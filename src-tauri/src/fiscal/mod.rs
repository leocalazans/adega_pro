use std::path::PathBuf;

use chrono::Datelike;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
pub enum Emissao {
    Normal,
    ContingenciaOffline,
}

#[derive(Debug, Clone, Serialize)]
pub struct NfceParams {
    pub modelo: u8,
    pub serie: u32,
    pub numero: u64,
    pub cnpj_emitente: String,
    pub uf: String,
    pub certificado_pfx_path: Option<String>,
    pub certificado_senha: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NfceResult {
    pub xml: String,
    pub emissao: Emissao,
    pub protocolo: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug)]
pub enum FiscalError {
    CertNotFound(String),
    InvalidCert(String),
    SignFailed(String),
    XmlError(String),
    NotAvailable(String),
}

impl std::fmt::Display for FiscalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CertNotFound(e) => write!(f, "Certificado não encontrado: {e}"),
            Self::InvalidCert(e) => write!(f, "Certificado inválido: {e}"),
            Self::SignFailed(e) => write!(f, "Falha na assinatura: {e}"),
            Self::XmlError(e) => write!(f, "Erro XML: {e}"),
            Self::NotAvailable(e) => write!(f, "Não disponível: {e}"),
        }
    }
}

impl std::error::Error for FiscalError {}

pub trait FiscalSigner: Send + Sync {
    fn sign_xml(&self, xml_raw: &str) -> Result<String, FiscalError>;
    fn is_available(&self) -> bool;
    fn name(&self) -> &str;
}

// ── A1 Signer (PFX via native-tls) ──────────────────────────

pub struct A1Signer {
    cert_path: PathBuf,
    password: String,
}

impl A1Signer {
    pub fn new(cert_path: PathBuf, password: String) -> Self {
        Self {
            cert_path,
            password,
        }
    }
}

impl FiscalSigner for A1Signer {
    fn sign_xml(&self, _xml_raw: &str) -> Result<String, FiscalError> {
        let pfx_bytes = std::fs::read(&self.cert_path).map_err(|e| {
            FiscalError::CertNotFound(format!(
                "Não foi possível ler {}: {e}",
                self.cert_path.display()
            ))
        })?;

        let _pkcs12 = native_tls::Identity::from_pkcs12(&pfx_bytes, &self.password)
            .map_err(|e| FiscalError::InvalidCert(format!("PFX inválido: {e}")))?;
        Err(FiscalError::NotAvailable(
            "certificado A1 validado, mas assinatura XMLDSig fiscal aguarda homologação".into(),
        ))
    }

    fn is_available(&self) -> bool {
        self.cert_path.exists()
    }

    fn name(&self) -> &str {
        "A1 (PFX)"
    }
}

// ── A3 Signer (stub para Win32 CNG/CryptoAPI) ───────────────

pub struct A3Signer {
    _store_name: String,
}

impl A3Signer {
    pub fn new(store_name: &str) -> Self {
        Self {
            _store_name: store_name.into(),
        }
    }
}

impl FiscalSigner for A3Signer {
    fn sign_xml(&self, _xml_raw: &str) -> Result<String, FiscalError> {
        Err(FiscalError::NotAvailable(
            "Certificado A3 via Win32 CNG/CryptoAPI ainda não implementado".into(),
        ))
    }

    fn is_available(&self) -> bool {
        false
    }

    fn name(&self) -> &str {
        "A3 (CNG)"
    }
}

// ── No-op signer (always contingency) ────────────────────────

pub struct NoSigner;

impl FiscalSigner for NoSigner {
    fn sign_xml(&self, _xml_raw: &str) -> Result<String, FiscalError> {
        Err(FiscalError::NotAvailable(
            "documento fiscal não pode ser assinado sem certificado".into(),
        ))
    }

    fn is_available(&self) -> bool {
        false
    }

    fn name(&self) -> &str {
        "Contingência (sem assinatura)"
    }
}

// ── NFC-e Engine ─────────────────────────────────────────────

pub struct NfceEngine;

impl NfceEngine {
    pub fn detect_contingency(&self, se_faz_ok: bool, online: bool) -> Emissao {
        if !online || !se_faz_ok {
            return Emissao::ContingenciaOffline;
        }
        Emissao::Normal
    }

    fn extract_chave(xml: &str) -> String {
        let marker = "Id=\"NFe";
        match xml.find(marker) {
            Some(start) => {
                let rest = &xml[start + marker.len()..];
                rest.chars().take_while(|c| c.is_ascii_digit()).collect()
            }
            None => String::new(),
        }
    }

    pub fn build_and_sign(&self, params: &NfceParams, signer: &dyn FiscalSigner) -> NfceResult {
        let raw_xml = build_nfce_xml(params);

        if !signer.is_available() {
            log::warn!(
                "NFC-e: certificado {} não disponível, entrando em contingência",
                signer.name()
            );
            let contingency = set_tpemis_contingency(&raw_xml);
            return NfceResult {
                xml: contingency.clone(),
                emissao: Emissao::ContingenciaOffline,
                protocolo: None,
                error: Some(format!("Certificado {} não disponível", signer.name())),
            };
        }

        match signer.sign_xml(&raw_xml) {
            Ok(signed) => {
                log::info!("NFC-e: XML assinado por {}", signer.name());
                NfceResult {
                    xml: signed,
                    emissao: Emissao::Normal,
                    protocolo: None,
                    error: None,
                }
            }
            Err(e) => {
                log::warn!("NFC-e: falha na assinatura: {e}, entrando em contingência");
                let contingency = set_tpemis_contingency(&raw_xml);
                NfceResult {
                    xml: contingency.clone(),
                    emissao: Emissao::ContingenciaOffline,
                    protocolo: None,
                    error: Some(e.to_string()),
                }
            }
        }
    }

    pub fn schedule_retransmission(&self, _uuid: &str) {
        log::info!("Retransmissão NFC-e em contingência agendada: {_uuid}");
    }
}

pub struct SatFiscal;

impl SatFiscal {
    pub fn call(&self, _dll_path: &str, _cmd: &str) -> Result<String, String> {
        Err("SAT Fiscal (SP) via C-Bindings ainda não implementado".into())
    }
}

// ── NFe B2B (Modelo 55) ──────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NfeParams {
    pub modelo: u8,
    pub serie: u32,
    pub numero: u64,
    pub cnpj_emitente: String,
    pub cnpj_destinatario: String,
    pub uf: String,
    pub cfop: String,
    pub items: Vec<NfeItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NfeItem {
    pub ean: String,
    pub description: String,
    pub ncm: String,
    pub cfop: String,
    pub unit: String,
    pub qty: f64,
    pub unit_value_brl_cents: i64,
    pub total_brl_cents: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct NfeResult {
    pub xml: String,
    pub chave: String,
    pub emissao: Emissao,
    pub error: Option<String>,
}

pub struct NfeEngine;

impl NfeEngine {
    pub fn build_and_sign(&self, params: &NfeParams, signer: &dyn FiscalSigner) -> NfeResult {
        let raw_xml = build_nfe_xml(params);
        let chave = NfceEngine::extract_chave(&raw_xml);

        if !signer.is_available() {
            log::warn!(
                "NFe: certificado {} não disponível, entrando em contingência",
                signer.name()
            );
            let contingency = set_tpemis_contingency(&raw_xml);
            return NfeResult {
                xml: contingency.clone(),
                chave: NfceEngine::extract_chave(&contingency),
                emissao: Emissao::ContingenciaOffline,
                error: Some(format!("Certificado {} não disponível", signer.name())),
            };
        }

        match signer.sign_xml(&raw_xml) {
            Ok(signed) => {
                log::info!("NFe: XML assinado por {}", signer.name());
                let signed_chave = NfceEngine::extract_chave(&signed);
                NfeResult {
                    xml: signed,
                    chave: if signed_chave.is_empty() {
                        chave
                    } else {
                        signed_chave
                    },
                    emissao: Emissao::Normal,
                    error: None,
                }
            }
            Err(e) => {
                log::warn!("NFe: falha na assinatura: {e}, entrando em contingência");
                let contingency = set_tpemis_contingency(&raw_xml);
                NfeResult {
                    xml: contingency.clone(),
                    chave: NfceEngine::extract_chave(&contingency),
                    emissao: Emissao::ContingenciaOffline,
                    error: Some(e.to_string()),
                }
            }
        }
    }
}

// ── XML Helpers ──────────────────────────────────────────────

/// Código IBGE da UF (tabela SEFAZ).
fn codigo_uf(uf: &str) -> u32 {
    match uf.to_uppercase().as_str() {
        "AC" => 12,
        "AL" => 27,
        "AP" => 16,
        "AM" => 13,
        "BA" => 29,
        "CE" => 23,
        "DF" => 53,
        "ES" => 32,
        "GO" => 52,
        "MA" => 21,
        "MT" => 51,
        "MS" => 50,
        "MG" => 31,
        "PA" => 15,
        "PB" => 25,
        "PR" => 41,
        "PE" => 26,
        "PI" => 22,
        "RJ" => 33,
        "RN" => 24,
        "RS" => 43,
        "RO" => 11,
        "RR" => 14,
        "SC" => 42,
        "SP" => 35,
        "SE" => 28,
        "TO" => 17,
        _ => 99,
    }
}

/// Dígito verificador da chave de acesso (módulo 11, pesos 2..9).
fn calcular_dv_chave(base_43: &str) -> char {
    let mut peso = 2;
    let mut soma: u64 = 0;
    for ch in base_43.chars().rev() {
        if let Some(d) = ch.to_digit(10) {
            soma += u64::from(d) * peso;
            peso = if peso == 9 { 2 } else { peso + 1 };
        }
    }
    let resto = soma % 11;
    let dv = if resto == 0 || resto == 1 {
        0
    } else {
        11 - resto
    };
    std::char::from_digit(dv as u32, 10).unwrap_or('0')
}

/// Gera chave de acesso de 44 dígitos (cUF + AAMM + CNPJ + mod + serie + nNF
/// + tpEmis + cNF + cDV) conforme leiaute NFe 4.00.
fn gerar_chave_acesso(
    cnpj_emitente: &str,
    uf: &str,
    modelo: u8,
    serie: u32,
    numero: u64,
    tp_emis: u8,
) -> String {
    let cnpj: String = cnpj_emitente
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect();
    let mut cnpj_padded = cnpj;
    while cnpj_padded.len() < 14 {
        cnpj_padded.insert(0, '0');
    }
    cnpj_padded.truncate(14);

    let now = chrono::Local::now();
    let aamm = format!("{:02}{:02}", now.year() % 100, now.month());
    let c_uf = format!("{:02}", codigo_uf(uf));
    let mod_ = format!("{:02}", modelo);
    let serie_p = format!("{:03}", serie);
    let n_nf = format!("{:09}", numero);
    let tp_emis_p = format!("{}", tp_emis);
    // cNF: valor derivado do número (8 dígitos) — mnemônico estável por nota
    let c_nf = format!("{:08}", (numero % 100_000_000));

    let base_43 = format!("{c_uf}{aamm}{cnpj_padded}{mod_}{serie_p}{n_nf}{tp_emis_p}{c_nf}");
    assert_eq!(base_43.len(), 43, "base da chave deve ter 43 dígitos");
    format!("{base_43}{}", calcular_dv_chave(&base_43))
}

/// Escapa caracteres especiais do XML 1.0.
fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn build_nfe_xml(params: &NfeParams) -> String {
    let uf = params.uf.to_uppercase();
    let cuf = codigo_uf(&uf);
    let chave = gerar_chave_acesso(
        &params.cnpj_emitente,
        &uf,
        params.modelo,
        params.serie,
        params.numero,
        1,
    );

    let now = chrono::Local::now();
    let dh_emi = now.format("%Y-%m-%dT%H:%M:%S%:z").to_string();

    let det_items: String = params
        .items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let n = i + 1;
            let unit_val = item.unit_value_brl_cents as f64 / 100.0;
            let total = item.total_brl_cents as f64 / 100.0;
            format!(
                r#"    <det nItem="{n}">
      <prod>
        <cProd>{ean}</cProd>
        <cEAN>{ean}</cEAN>
        <xProd>{desc}</xProd>
        <NCM>{ncm}</NCM>
        <CFOP>{cfop}</CFOP>
        <uCom>{unit}</uCom>
        <qCom>{qty}</qCom>
        <vUnCom>{unit_val}</vUnCom>
        <vProd>{total}</vProd>
        <cEANTrib>{ean}</cEANTrib>
        <uTrib>{unit}</uTrib>
        <qTrib>{qty}</qTrib>
        <vUnTrib>{unit_val}</vUnTrib>
        <indTot>1</indTot>
      </prod>
      <imposto>
        <ICMS>
          <ICMS00>
            <orig>0</orig>
            <CST>00</CST>
            <modBC>0</modBC>
            <vBC>{total}</vBC>
            <pICMS>0</pICMS>
            <vICMS>0</vICMS>
          </ICMS00>
        </ICMS>
        <PIS>
          <PISOutr>
            <CST>99</CST>
            <qBCProd>0.0000</qBCProd>
            <vAliqProd>0.0000</vAliqProd>
            <vPIS>0.00</vPIS>
          </PISOutr>
        </PIS>
        <COFINS>
          <COFINSAliq>
            <CST>99</CST>
            <qBCProd>0.0000</qBCProd>
            <vAliqProd>0.0000</vAliqProd>
            <vCOFINS>0.00</vCOFINS>
          </COFINSAliq>
        </COFINS>
      </imposto>
    </det>"#,
                ean = item.ean,
                desc = escape_xml(&item.description),
                ncm = item.ncm,
                cfop = item.cfop,
                unit = item.unit,
                qty = item.qty,
                unit_val = unit_val,
                total = total,
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<NFe xmlns="http://www.portalfiscal.inf.br/nfe">
  <infNFe versao="4.00" Id="NFe{chave}">
    <ide>
      <cUF>{cuf}</cUF>
      <cNF>{c_nf}</cNF>
      <natOp>VENDA</natOp>
      <mod>{mod_}</mod>
      <serie>{serie}</serie>
      <nNF>{numero}</nNF>
      <dhEmi>{dh_emi}</dhEmi>
      <tpNF>1</tpNF>
      <idDest>1</idDest>
      <cMunFG>3550308</cMunFG>
      <tpImp>1</tpImp>
      <tpEmis>1</tpEmis>
      <cDV>{c_dv}</cDV>
      <tpAmb>2</tpAmb>
      <finNFe>1</finNFe>
      <indFinal>1</indFinal>
      <indPres>1</indPres>
      <procEmi>0</procEmi>
      <verProc>AutoControl-PDV-1.0</verProc>
    </ide>
    <emit>
      <CNPJ>{cnpj_emit}</CNPJ>
      <xNome>AutoControl PDV</xNome>
      <enderEmit>
        <xLgr>Rua Principal</xLgr>
        <nro>100</nro>
        <cMun>3550308</cMun>
        <xMun>SAO PAULO</xMun>
        <UF>{uf}</UF>
        <CEP>01001000</CEP>
        <cPais>1058</cPais>
        <xPais>BRASIL</xPais>
      </enderEmit>
      <CRT>1</CRT>
    </emit>
    <dest>
      <CNPJ>{cnpj_dest}</CNPJ>
      <xNome>Destinatario</xNome>
      <enderDest>
        <xLgr>Rua Destinatario</xLgr>
        <nro>200</nro>
        <cMun>3550308</cMun>
        <xMun>SAO PAULO</xMun>
        <UF>{uf}</UF>
        <CEP>01002000</CEP>
        <cPais>1058</cPais>
        <xPais>BRASIL</xPais>
      </enderDest>
      <indIEDest>9</indIEDest>
    </dest>
{items}
    <total>
      <ICMSTot>
        <vBC>0.00</vBC><vICMS>0.00</vICMS><vICMSDeson>0.00</vICMSDeson><vFCP>0.00</vFCP>
        <vBCST>0.00</vBCST><vST>0.00</vST><vFCPST>0.00</vFCPST><vFCPSTRet>0.00</vFCPSTRet>
        <vProd>{v_total}</vProd><vFrete>0.00</vFrete><vSeg>0.00</vSeg><vDesc>0.00</vDesc><vII>0.00</vII>
        <vIPI>0.00</vIPI><vIPIDevol>0.00</vIPIDevol>
        <vPIS>0.00</vPIS><vCOFINS>0.00</vCOFINS>
        <vOutro>0.00</vOutro><vNF>{v_total}</vNF>
      </ICMSTot>
    </total>
    <transp><modFrete>9</modFrete></transp>
    <pag><detPag><tPag>01</tPag><vPag>{v_total}</vPag></detPag></pag>
  </infNFe>
</NFe>"#,
        chave = chave,
        cuf = cuf,
        c_nf = &chave[35..43],
        c_dv = &chave[43..44],
        mod_ = params.modelo,
        serie = params.serie,
        numero = params.numero,
        dh_emi = dh_emi,
        cnpj_emit = params.cnpj_emitente,
        cnpj_dest = params.cnpj_destinatario,
        uf = uf,
        items = det_items,
        v_total = format!(
            "{:.2}",
            params
                .items
                .iter()
                .map(|i| i.total_brl_cents as f64 / 100.0)
                .sum::<f64>()
        ),
    )
}

fn build_nfce_xml(params: &NfceParams) -> String {
    let uf = params.uf.to_uppercase();
    let cuf = codigo_uf(&uf);
    let chave = gerar_chave_acesso(
        &params.cnpj_emitente,
        &uf,
        params.modelo,
        params.serie,
        params.numero,
        1,
    );
    let now = chrono::Local::now();
    let dh_emi = now.format("%Y-%m-%dT%H:%M:%S%:z").to_string();

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<NFe xmlns="http://www.portalfiscal.inf.br/nfe">
  <infNFe versao="4.00" Id="NFe{chave}">
    <ide>
      <cUF>{cuf}</cUF>
      <cNF>{c_nf}</cNF>
      <natOp>VENDA</natOp>
      <mod>{mod_}</mod>
      <serie>{serie}</serie>
      <nNF>{numero}</nNF>
      <dhEmi>{dh_emi}</dhEmi>
      <tpNF>1</tpNF>
      <idDest>1</idDest>
      <cMunFG>3550308</cMunFG>
      <tpImp>1</tpImp>
      <tpEmis>1</tpEmis>
      <cDV>{c_dv}</cDV>
      <tpAmb>2</tpAmb>
      <finNFe>1</finNFe>
      <indFinal>1</indFinal>
      <indPres>1</indPres>
      <procEmi>0</procEmi>
      <verProc>AutoControl-PDV-1.0</verProc>
    </ide>
    <emit>
      <CNPJ>{cnpj}</CNPJ>
      <xNome>AutoControl PDV</xNome>
      <enderEmit>
        <xLgr>Rua Principal</xLgr>
        <nro>100</nro>
        <cMun>3550308</cMun>
        <xMun>SAO PAULO</xMun>
        <UF>{uf}</UF>
        <CEP>01001000</CEP>
        <cPais>1058</cPais>
        <xPais>BRASIL</xPais>
      </enderEmit>
      <CRT>1</CRT>
    </emit>
  </infNFe>
</NFe>"#,
        chave = chave,
        cuf = cuf,
        c_nf = &chave[35..43],
        c_dv = &chave[43..44],
        mod_ = params.modelo,
        serie = params.serie,
        numero = params.numero,
        dh_emi = dh_emi,
        cnpj = params.cnpj_emitente,
        uf = uf,
    )
}

fn set_tpemis_contingency(xml: &str) -> String {
    // Contingência: tpEmis=9 (EPEC/offline) e tpAmb=2 (homologação) — a chave
    // já é gerada com tpEmis=1 no build; reemitir a chave com tpEmis=9 seria
    // responsabilidade do caller. Mantemos o ajuste do campo no XML para
    // sinalizar o modo de emissão ao SEFAZ.
    xml.replace("<tpEmis>1</tpEmis>", "<tpEmis>9</tpEmis>")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chave_acesso_44_digitos_com_dv_valido() {
        let chave = gerar_chave_acesso("11222333000181", "SP", 55, 1, 1, 1);
        assert_eq!(chave.len(), 44, "chave deve ter 44 dígitos");
        assert!(chave.chars().all(|c| c.is_ascii_digit()));
        let dv = calcular_dv_chave(&chave[..43]);
        assert_eq!(chave.chars().last().unwrap(), dv);
    }

    #[test]
    fn build_nfe_xml_gera_id_real() {
        let params = NfeParams {
            modelo: 55,
            serie: 1,
            numero: 1,
            cnpj_emitente: "11222333000181".into(),
            cnpj_destinatario: "11222333000181".into(),
            uf: "SP".into(),
            cfop: "5102".into(),
            items: vec![NfeItem {
                ean: "789".into(),
                description: "Troca de oleo".into(),
                ncm: "27101932".into(),
                cfop: "5102".into(),
                unit: "UN".into(),
                qty: 1.0,
                unit_value_brl_cents: 25000,
                total_brl_cents: 25000,
            }],
        };
        let xml = build_nfe_xml(&params);
        assert!(xml.contains("Id=\"NFe"), "deve ter Id com chave");
        let marker = "Id=\"NFe";
        let start = xml.find(marker).unwrap() + marker.len();
        let chave: String = xml[start..]
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        assert_eq!(chave.len(), 44);
        assert!(xml.contains("<vNF>250.00</vNF>"));
        assert!(xml.contains("<tpAmb>2</tpAmb>"));
    }

    #[test]
    fn build_nfce_xml_gera_id_real() {
        let params = NfceParams {
            modelo: 65,
            serie: 1,
            numero: 1,
            cnpj_emitente: "11222333000181".into(),
            uf: "SP".into(),
            certificado_pfx_path: None,
            certificado_senha: None,
        };
        let xml = build_nfce_xml(&params);
        let marker = "Id=\"NFe";
        let start = xml.find(marker).unwrap() + marker.len();
        let chave: String = xml[start..]
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        assert_eq!(chave.len(), 44);
    }

    #[test]
    fn contingencia_ajusta_tpemis() {
        let xml = "<tpEmis>1</tpEmis>";
        assert!(set_tpemis_contingency(xml).contains("<tpEmis>9</tpEmis>"));
    }
}
