use std::collections::HashMap;
use std::path::Path;
use std::sync::{Mutex, RwLock};

use rusqlite::{Connection, OptionalExtension};
use serde::Deserialize;
use serde::Serialize;

pub mod delivery;

pub struct Db {
    conn: Mutex<Connection>,
    product_cache: RwLock<HashMap<String, Product>>,
}

fn normalize_consumer_cpf(value: Option<&str>) -> rusqlite::Result<Option<String>> {
    let Some(value) = value.map(str::trim).filter(|v| !v.is_empty()) else {
        return Ok(None);
    };
    let invalid = || rusqlite::Error::InvalidParameterName("CPF informado é inválido".into());
    if !value
        .bytes()
        .all(|b| b.is_ascii_digit() || matches!(b, b'.' | b'-'))
    {
        return Err(invalid());
    }
    let digits: Vec<u32> = value
        .bytes()
        .filter(u8::is_ascii_digit)
        .map(|b| (b - b'0') as u32)
        .collect();
    if digits.len() != 11 || digits.iter().all(|d| *d == digits[0]) {
        return Err(invalid());
    }
    for size in [9, 10] {
        let sum: u32 = digits[..size]
            .iter()
            .enumerate()
            .map(|(i, d)| d * (size + 1 - i) as u32)
            .sum();
        let check = ((sum * 10) % 11) % 10;
        if check != digits[size] {
            return Err(invalid());
        }
    }
    Ok(Some(value.chars().filter(char::is_ascii_digit).collect()))
}

#[derive(Debug, Clone, Serialize)]
pub struct Product {
    pub id: i64,
    pub ean: String,
    pub part_number: String,
    pub description: String,
    pub brand: Option<String>,
    pub price_brl_cents: i64,
    pub stock_qty: f64,
    pub updated_at: i64,
    pub fractional_allowed: bool,
    pub multiplier_factor: f64,
    pub unit_type: Option<String>,
    pub min_stock: f64,
    pub active: bool,
    pub image_url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Employee {
    pub id: i64,
    pub name: String,
    pub role: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TimeEntry {
    pub id: i64,
    pub employee_id: i64,
    pub employee_name: String,
    pub event_type: String,
    pub note: Option<String>,
    pub occurred_at: i64,
}

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: i64,
    pub employee_id: Option<i64>,
    pub display_name: String,
    pub username: String,
    pub role: String,
    pub password_hash: Option<String>,
    pub must_change_password: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Expense {
    pub id: i64,
    pub description: String,
    pub category: String,
    pub amount_brl_cents: i64,
    pub due_date: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Supplier {
    pub id: i64,
    pub name: String,
    pub document: Option<String>,
    pub phone: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Promotion {
    pub id: i64,
    pub title: String,
    pub subtitle: Option<String>,
    pub price_label: String,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DashboardSummary {
    pub sales_today_brl_cents: i64,
    pub items_today: i64,
    pub new_customers_today: i64,
    pub low_stock_count: i64,
    pub daily_sales: Vec<i64>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct PaymentTotals {
    pub cash_brl_cents: i64,
    pub card_brl_cents: i64,
    pub pix_brl_cents: i64,
    pub other_brl_cents: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SalesReportRow {
    pub uuid: String,
    pub total_brl_cents: i64,
    pub payment_method: String,
    pub terminal_id: String,
    pub items_count: i64,
    pub created_at: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SaleItemIn {
    pub ean: String,
    pub qty: f64,
    pub price_brl_cents: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct PaymentIn {
    pub method: String,
    pub amount_brl_cents: i64,
    #[serde(default)]
    pub extra: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SaleOut {
    pub sale_uuid: String,
    pub total_brl_cents: i64,
    pub payment_method: String,
    pub items_count: usize,
    pub outbox_id: i64,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct OutboxRow {
    pub id: i64,
    pub uuid: String,
    pub entity: String,
    pub operation: String,
    pub payload: String,
    pub status: String,
    pub attempts: i64,
    pub last_error: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CashStatus {
    pub open: bool,
    pub session_id: Option<i64>,
    pub opened_at: Option<i64>,
    pub opening_brl_cents: i64,
    pub sales_brl_cents: i64,
    pub expected_brl_cents: i64,
    pub closed_at: Option<i64>,
    pub closing_brl_cents: Option<i64>,
    pub difference_brl_cents: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashMovement {
    pub id: i64,
    pub session_id: i64,
    pub movement_type: String,
    pub amount_brl_cents: i64,
    pub description: Option<String>,
    pub operator: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashCounted {
    pub cash_brl_cents: i64,
    pub card_brl_cents: i64,
    pub pix_brl_cents: i64,
    pub cheque_brl_cents: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct FechamentoReport {
    pub session: CashStatus,
    pub movements: Vec<CashMovement>,
    pub sales_by_method: serde_json::Value,
    pub expected_total: i64,
    pub counted_total: i64,
    pub difference: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PrintSpoolRow {
    pub id: i64,
    pub sale_uuid: String,
    pub receipt_json: String,
    pub status: String,
    pub attempts: i64,
    pub last_error: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Customer {
    pub id: i64,
    pub name: String,
    pub cpf_cnpj: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub points: i64,
    pub total_spent_brl_cents: i64,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointsHistory {
    pub id: i64,
    pub customer_id: i64,
    pub points: i64,
    pub sale_uuid: Option<String>,
    pub description: Option<String>,
    pub created_at: i64,
}

const MIGRATIONS: &[(&str, &str)] = &[
    (
        "1",
        "CREATE TABLE IF NOT EXISTS products (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            ean TEXT NOT NULL UNIQUE,
            part_number TEXT NOT NULL UNIQUE,
            description TEXT NOT NULL,
            brand TEXT,
            price_brl_cents INTEGER NOT NULL DEFAULT 0,
            stock_qty REAL NOT NULL DEFAULT 0,
            updated_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_products_ean ON products(ean);
        CREATE INDEX IF NOT EXISTS idx_products_part ON products(part_number);
        CREATE INDEX IF NOT EXISTS idx_products_desc ON products(description COLLATE NOCASE);

        CREATE TABLE IF NOT EXISTS sales (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            uuid TEXT NOT NULL UNIQUE,
            total_brl_cents INTEGER NOT NULL,
            payment_method TEXT NOT NULL,
            items_json TEXT NOT NULL,
            created_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS sync_outbox (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            uuid TEXT NOT NULL,
            entity TEXT NOT NULL,
            operation TEXT NOT NULL,
            payload TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            attempts INTEGER NOT NULL DEFAULT 0,
            last_error TEXT,
            created_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_outbox_status ON sync_outbox(status, created_at);
        ",
    ),
    (
        "2",
        "CREATE TABLE IF NOT EXISTS cash_sessions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            opened_at INTEGER NOT NULL,
            opening_brl_cents INTEGER NOT NULL DEFAULT 0,
            closed_at INTEGER,
            closing_brl_cents INTEGER,
            expected_brl_cents INTEGER,
            difference_brl_cents INTEGER
        );
        CREATE INDEX IF NOT EXISTS idx_cash_open ON cash_sessions(closed_at);
        ",
    ),
    (
        "3",
        "CREATE TABLE IF NOT EXISTS print_spool_queue (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            sale_uuid TEXT NOT NULL,
            receipt_json TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            attempts INTEGER NOT NULL DEFAULT 0,
            last_error TEXT,
            created_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_print_spool ON print_spool_queue(status, created_at);

        CREATE TABLE IF NOT EXISTS cash_movements (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id INTEGER NOT NULL REFERENCES cash_sessions(id),
            type TEXT NOT NULL,
            amount_brl_cents INTEGER NOT NULL,
            description TEXT,
            operator TEXT,
            created_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_cash_mov_session ON cash_movements(session_id, type);

        ALTER TABLE products ADD COLUMN fractional_allowed INTEGER NOT NULL DEFAULT 0;
        ALTER TABLE products ADD COLUMN multiplier_factor REAL NOT NULL DEFAULT 1.0;
        ALTER TABLE products ADD COLUMN unit_type TEXT;
        ",
    ),
    (
        "4",
        "ALTER TABLE sales ADD COLUMN terminal_id TEXT NOT NULL DEFAULT 'default';

        CREATE TABLE IF NOT EXISTS customers (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            cpf_cnpj TEXT UNIQUE,
            phone TEXT,
            email TEXT,
            points INTEGER NOT NULL DEFAULT 0,
            total_spent_brl_cents INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_customers_cpf ON customers(cpf_cnpj);

        CREATE TABLE IF NOT EXISTS points_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            customer_id INTEGER NOT NULL REFERENCES customers(id),
            points INTEGER NOT NULL,
            sale_uuid TEXT,
            description TEXT,
            created_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_points_customer ON points_history(customer_id, created_at);
        ",
    ),
    (
        "5",
        "DROP INDEX IF EXISTS idx_products_ean;
        DROP INDEX IF EXISTS idx_products_part;

        CREATE INDEX IF NOT EXISTS idx_sales_created_payment
            ON sales(created_at, payment_method, total_brl_cents);
        CREATE INDEX IF NOT EXISTS idx_products_brand
            ON products(brand COLLATE NOCASE);
        CREATE INDEX IF NOT EXISTS idx_customers_name
            ON customers(name COLLATE NOCASE);
        CREATE INDEX IF NOT EXISTS idx_customers_phone
            ON customers(phone);
        ",
    ),
    (
        "6",
        "ALTER TABLE products ADD COLUMN min_stock REAL NOT NULL DEFAULT 10;

        CREATE TABLE IF NOT EXISTS employees (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            role TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'Ativo',
            updated_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS expenses (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            description TEXT NOT NULL,
            category TEXT NOT NULL,
            amount_brl_cents INTEGER NOT NULL,
            due_date TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'Pendente',
            updated_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS suppliers (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            document TEXT,
            phone TEXT,
            status TEXT NOT NULL DEFAULT 'Ativo',
            updated_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS promotions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            subtitle TEXT,
            price_label TEXT NOT NULL,
            active INTEGER NOT NULL DEFAULT 1,
            updated_at INTEGER NOT NULL
        );

        INSERT OR IGNORE INTO products(ean,part_number,description,brand,price_brl_cents,stock_qty,updated_at,min_stock) VALUES
          ('78900001','PROD001','Vinho Tinto Suave','Adega',3000,8,strftime('%s','now'),10),
          ('78900002','PROD002','Cerveja Artesanal IPA','Commerce',1500,40,strftime('%s','now'),25),
          ('78900003','PROD003','Whisky 12 Anos','Reserva',12000,15,strftime('%s','now'),5),
          ('78900004','PROD004','Gin Importado','London',13000,12,strftime('%s','now'),10),
          ('78900005','PROD005','Água Tônica','Fresh',500,3,strftime('%s','now'),20),
          ('78900006','PROD006','Energético','Power',800,50,strftime('%s','now'),20),
          ('78900007','PROD007','Saca-rolhas','Casa',2500,2,strftime('%s','now'),5),
          ('78900008','PROD008','Cerveja Pilsen Pack 6','Commerce',2200,11,strftime('%s','now'),15);
        INSERT INTO employees(name,role,status,updated_at) SELECT 'Ana Silva','Caixa','Ativo',strftime('%s','now') WHERE NOT EXISTS(SELECT 1 FROM employees);
        INSERT INTO employees(name,role,status,updated_at) SELECT 'Bruno Costa','Gerente','Ativo',strftime('%s','now') WHERE (SELECT COUNT(*) FROM employees)=1;
        INSERT INTO employees(name,role,status,updated_at) SELECT 'Carlos Dias','Caixa','Inativo',strftime('%s','now') WHERE (SELECT COUNT(*) FROM employees)=2;
        INSERT INTO expenses(description,category,amount_brl_cents,due_date,status,updated_at) SELECT 'Pagamento de salário','Folha',180000,date('now','+5 day'),'Pago',strftime('%s','now') WHERE NOT EXISTS(SELECT 1 FROM expenses);
        INSERT INTO expenses(description,category,amount_brl_cents,due_date,status,updated_at) SELECT 'Conta de energia','Contas fixas',45075,date('now','+10 day'),'Pendente',strftime('%s','now') WHERE (SELECT COUNT(*) FROM expenses)=1;
        INSERT INTO expenses(description,category,amount_brl_cents,due_date,status,updated_at) SELECT 'Aluguel','Aluguel',350000,date('now','+8 day'),'Pendente',strftime('%s','now') WHERE (SELECT COUNT(*) FROM expenses)=2;
        INSERT INTO suppliers(name,document,phone,status,updated_at) SELECT 'Distribuidora Commerce','12.345.678/0001-90','(11) 3333-1000','Ativo',strftime('%s','now') WHERE NOT EXISTS(SELECT 1 FROM suppliers);
        INSERT INTO promotions(title,subtitle,price_label,active,updated_at) SELECT 'Cerveja Pilsen Pack 6','Oferta da semana','R$ 19,99',1,strftime('%s','now') WHERE NOT EXISTS(SELECT 1 FROM promotions);
        INSERT INTO promotions(title,subtitle,price_label,active,updated_at) SELECT 'Vinho Tinto Suave','Leve 3, pague 2','Pague 2 Leve 3',1,strftime('%s','now') WHERE (SELECT COUNT(*) FROM promotions)=1;
        INSERT INTO customers(name,cpf_cnpj,phone,email,points,total_spent_brl_cents,created_at) SELECT 'Cliente Demonstração','000.000.000-00','(11) 99999-0000','cliente@exemplo.local',120,0,strftime('%s','now') WHERE NOT EXISTS(SELECT 1 FROM customers);
        CREATE INDEX IF NOT EXISTS idx_products_low_stock ON products(stock_qty,min_stock);
        CREATE INDEX IF NOT EXISTS idx_expenses_status_due ON expenses(status,due_date);
        ",
    ),
    (
        "7",
        "CREATE TABLE IF NOT EXISTS purchase_orders (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            product_ean TEXT NOT NULL REFERENCES products(ean),
            quantity REAL NOT NULL CHECK(quantity > 0),
            status TEXT NOT NULL DEFAULT 'Pendente',
            created_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_purchase_orders_status_created ON purchase_orders(status,created_at);",
    ),
    (
        "8",
        "ALTER TABLE products ADD COLUMN active INTEGER NOT NULL DEFAULT 1;
         CREATE INDEX IF NOT EXISTS idx_products_active_desc ON products(active,description COLLATE NOCASE);",
    ),
    (
        "9",
        "CREATE TABLE IF NOT EXISTS app_settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS license_audit (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            action TEXT NOT NULL,
            detail TEXT,
            created_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS fiscal_queue (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            sale_uuid TEXT NOT NULL UNIQUE,
            consumer_document TEXT,
            model INTEGER NOT NULL DEFAULT 65,
            status TEXT NOT NULL DEFAULT 'pending_configuration',
            access_key TEXT,
            xml TEXT,
            protocol TEXT,
            attempts INTEGER NOT NULL DEFAULT 0,
            last_error TEXT,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_fiscal_queue_status_created ON fiscal_queue(status,created_at);
        CREATE TABLE IF NOT EXISTS external_orders (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            provider TEXT NOT NULL,
            external_id TEXT NOT NULL,
            unit_id TEXT NOT NULL,
            status TEXT NOT NULL,
            payload_json TEXT NOT NULL,
            sale_uuid TEXT,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            UNIQUE(provider,external_id)
        );
        CREATE INDEX IF NOT EXISTS idx_external_orders_status ON external_orders(status,created_at);",
    ),
    (
        "10",
        "CREATE TABLE delivery_inbox (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tenant_id TEXT NOT NULL,
            unit_id TEXT NOT NULL,
            provider TEXT NOT NULL,
            merchant_id TEXT NOT NULL,
            event_id TEXT NOT NULL,
            order_id TEXT NOT NULL,
            payload_json TEXT NOT NULL,
            received_at INTEGER NOT NULL,
            UNIQUE(tenant_id, unit_id, provider, merchant_id, event_id)
        );
        CREATE INDEX idx_delivery_inbox_scope ON delivery_inbox(
            tenant_id, unit_id, provider, merchant_id, id
        );",
    ),
    (
        "11",
        "CREATE TABLE IF NOT EXISTS time_entries (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            employee_id INTEGER NOT NULL REFERENCES employees(id),
            event_type TEXT NOT NULL CHECK(event_type IN ('clock_in','break_start','break_end','clock_out')),
            note TEXT,
            occurred_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_time_entries_employee_date
            ON time_entries(employee_id, occurred_at DESC);",
    ),
    (
        "12",
        "CREATE TABLE IF NOT EXISTS app_users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            employee_id INTEGER REFERENCES employees(id),
            display_name TEXT NOT NULL,
            username TEXT NOT NULL UNIQUE COLLATE NOCASE,
            role TEXT NOT NULL CHECK(role IN ('superadmin','admin','manager','cashier','stock','finance','viewer')),
            password_hash TEXT,
            must_change_password INTEGER NOT NULL DEFAULT 1,
            active INTEGER NOT NULL DEFAULT 1,
            updated_at INTEGER NOT NULL
        );
        INSERT INTO app_users(display_name,username,role,password_hash,must_change_password,updated_at)
        SELECT 'Administrador','admin','admin',NULL,1,unixepoch()
        WHERE NOT EXISTS(SELECT 1 FROM app_users);",
    ),
    ("13", "ALTER TABLE products ADD COLUMN image_url TEXT;"),
];

impl Db {
    pub fn backup_to(&self, target: &Path) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("VACUUM INTO ?1", [target.to_string_lossy().as_ref()])?;
        Ok(())
    }

    pub fn validate_backup(path: &Path) -> Result<(), String> {
        let conn = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|e| e.to_string())?;
        let integrity: String = conn
            .query_row("PRAGMA integrity_check", [], |row| row.get(0))
            .map_err(|e| e.to_string())?;
        if integrity != "ok" {
            return Err(format!("Backup SQLite inválido: {integrity}"));
        }
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(|e| e.to_string())?;
        if version < 1 {
            return Err("O arquivo não é um backup CommerceCTRL".into());
        }
        Ok(())
    }

    pub fn open(path: &Path) -> rusqlite::Result<Db> {
        let conn = Connection::open(path)?;

        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "FULL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "busy_timeout", "5000")?;
        conn.pragma_update(None, "mmap_size", 268_435_456)?;
        conn.pragma_update(None, "cache_size", -20_000)?;
        conn.pragma_update(None, "wal_autocheckpoint", "1000")?;
        conn.pragma_update(None, "secure_delete", "ON")?;
        conn.pragma_update(None, "temp_store", "MEMORY")?;

        let integrity: String = conn.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
        if integrity != "ok" {
            log::error!("SQLite integrity_check falhou: {integrity}");
            conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE); REINDEX;")?;
            let recheck: String = conn.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
            if recheck != "ok" {
                log::error!("SQLite ainda corrompido após REINDEX: {recheck}");
                return Err(rusqlite::Error::InvalidParameterName(
                    "Banco de dados corrompido".into(),
                ));
            }
            log::info!("SQLite recuperado com REINDEX");
        }

        let db = Db {
            conn: Mutex::new(conn),
            product_cache: RwLock::new(HashMap::new()),
        };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> rusqlite::Result<()> {
        let mut conn = self.conn.lock().unwrap();
        Self::apply_migrations(&mut conn, MIGRATIONS)
    }

    fn apply_migrations(
        conn: &mut Connection,
        migrations: &[(&str, &str)],
    ) -> rusqlite::Result<()> {
        let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        for (next, sql) in migrations {
            let next: i64 = next.parse().unwrap();
            if version < next {
                log::info!("Rodando migration {next}");
                let tx = conn.transaction()?;
                tx.execute_batch(sql)?;
                tx.pragma_update(None, "user_version", next)?;
                tx.commit()?;
            }
        }
        Ok(())
    }

    fn product_from_row(row: &rusqlite::Row) -> rusqlite::Result<Product> {
        Ok(Product {
            id: row.get(0)?,
            ean: row.get(1)?,
            part_number: row.get(2)?,
            description: row.get(3)?,
            brand: row.get(4)?,
            price_brl_cents: row.get(5)?,
            stock_qty: row.get(6)?,
            updated_at: row.get(7)?,
            fractional_allowed: row.get::<_, i64>(8).unwrap_or(0) != 0,
            multiplier_factor: row.get::<_, f64>(9).unwrap_or(1.0),
            unit_type: row.get(10).ok(),
            min_stock: row.get::<_, f64>(11).unwrap_or(10.0),
            active: row.get::<_, i64>(12).unwrap_or(1) != 0,
            image_url: row.get(13).ok(),
        })
    }

    pub fn get_setting(&self, key: &str) -> rusqlite::Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT value FROM app_settings WHERE key=?1",
            [key],
            |row| row.get(0),
        )
        .optional()
    }

    pub fn auth_user(&self, username: &str) -> rusqlite::Result<Option<AuthUser>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id,employee_id,display_name,username,role,password_hash,must_change_password FROM app_users WHERE username=?1 COLLATE NOCASE AND active=1",
            [username.trim()], |row| Ok(AuthUser { id:row.get(0)?, employee_id:row.get(1)?, display_name:row.get(2)?, username:row.get(3)?, role:row.get(4)?, password_hash:row.get(5)?, must_change_password:row.get::<_,i64>(6)? != 0 })
        ).optional()
    }

    pub fn set_user_password(
        &self,
        user_id: i64,
        hash: &str,
        must_change: bool,
    ) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("UPDATE app_users SET password_hash=?1,must_change_password=?2,updated_at=unixepoch() WHERE id=?3", rusqlite::params![hash, must_change as i64, user_id])?;
        Ok(())
    }

    pub fn provision_user(
        &self,
        employee_id: i64,
        display_name: &str,
        username: &str,
        role: &str,
        password_hash: &str,
    ) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO app_users(employee_id,display_name,username,role,password_hash,must_change_password,active,updated_at)
             VALUES(?1,?2,?3,?4,?5,1,1,unixepoch())
             ON CONFLICT(username) DO UPDATE SET employee_id=excluded.employee_id,display_name=excluded.display_name,role=excluded.role,password_hash=excluded.password_hash,must_change_password=1,active=1,updated_at=unixepoch()",
            rusqlite::params![employee_id, display_name, username.trim(), role, password_hash],
        )?;
        Ok(())
    }

    pub fn set_setting(&self, key: &str, value: &str) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO app_settings(key,value,updated_at) VALUES(?1,?2,unixepoch()) ON CONFLICT(key) DO UPDATE SET value=excluded.value,updated_at=excluded.updated_at",
            rusqlite::params![key, value],
        )?;
        Ok(())
    }

    pub fn audit_license(&self, action: &str, detail: Option<&str>) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO license_audit(action,detail,created_at) VALUES(?1,?2,unixepoch())",
            rusqlite::params![action, detail],
        )?;
        Ok(())
    }

    const COLS: &'static str =
        "id, ean, part_number, description, brand, price_brl_cents, stock_qty, updated_at, \
         fractional_allowed, multiplier_factor, unit_type, min_stock, active, image_url";

    pub fn set_product_image(&self, ean: &str, image_url: Option<&str>) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE products SET image_url=?1,updated_at=unixepoch() WHERE ean=?2",
            rusqlite::params![image_url, ean],
        )?;
        drop(conn);
        self.product_cache.write().unwrap().clear();
        Ok(())
    }

    pub fn search_by_ean(&self, ean: &str) -> rusqlite::Result<Option<Product>> {
        let key = ean.trim();
        if let Some(product) = self.product_cache.read().unwrap().get(key).cloned() {
            return Ok(Some(product));
        }
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare_cached(&format!(
            "SELECT {} FROM products WHERE active=1 AND (ean = ?1 OR part_number = ?1) LIMIT 1",
            Self::COLS
        ))?;
        let result = stmt.query_row([key], Self::product_from_row).optional()?;
        drop(stmt);
        drop(conn);
        if let Some(product) = &result {
            let mut cache = self.product_cache.write().unwrap();
            cache.insert(product.ean.clone(), product.clone());
        }
        Ok(result)
    }

    pub fn search(&self, query: &str, limit: i64) -> rusqlite::Result<Vec<Product>> {
        let query = query.trim();
        if query.is_empty() {
            return self.list_products(limit, 0);
        }
        if let Some(product) = self.search_by_ean(query)? {
            return Ok(vec![product]);
        }
        let conn = self.conn.lock().unwrap();
        let prefix = format!("{query}%");
        let mut stmt = conn.prepare_cached(&format!(
            "SELECT {} FROM products
             WHERE active=1 AND (description LIKE ?1 COLLATE NOCASE OR brand LIKE ?1 COLLATE NOCASE)
             ORDER BY description COLLATE NOCASE LIMIT ?2",
            Self::COLS
        ))?;
        let rows = stmt.query_map(rusqlite::params![&prefix, limit], Self::product_from_row)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn list_products(&self, limit: i64, offset: i64) -> rusqlite::Result<Vec<Product>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(&format!(
            "SELECT {} FROM products WHERE active=1 ORDER BY description COLLATE NOCASE LIMIT ?1 OFFSET ?2",
            Self::COLS
        ))?;
        let rows = stmt.query_map(rusqlite::params![limit, offset], Self::product_from_row)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn adjust_stock(&self, ean: &str, delta: f64) -> rusqlite::Result<Option<f64>> {
        let conn = self.conn.lock().unwrap();
        let updated = conn.execute(
            "UPDATE products SET stock_qty = stock_qty + ?1, updated_at = ?2 WHERE ean = ?3",
            rusqlite::params![delta, chrono::Utc::now().timestamp(), ean],
        )?;
        if updated == 0 {
            return Ok(None);
        }
        let stock: f64 = conn.query_row(
            "SELECT stock_qty FROM products WHERE ean = ?1",
            [ean],
            |r| r.get(0),
        )?;
        drop(conn);
        self.product_cache.write().unwrap().clear();
        Ok(Some(stock))
    }

    pub fn set_product_active(&self, ean: &str, active: bool) -> rusqlite::Result<bool> {
        let conn = self.conn.lock().unwrap();
        let changed = conn.execute(
            "UPDATE products SET active=?1,updated_at=?2 WHERE ean=?3",
            rusqlite::params![active as i64, chrono::Utc::now().timestamp(), ean],
        )? == 1;
        drop(conn);
        self.product_cache.write().unwrap().clear();
        Ok(changed)
    }

    pub fn upsert_product(
        &self,
        ean: &str,
        part_number: &str,
        description: &str,
        brand: Option<&str>,
        price_brl_cents: i64,
        stock_qty: f64,
    ) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO products (ean, part_number, description, brand, price_brl_cents, stock_qty, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(ean) DO UPDATE SET
                part_number = excluded.part_number,
                description = excluded.description,
                brand = excluded.brand,
                price_brl_cents = excluded.price_brl_cents,
                stock_qty = excluded.stock_qty,
                updated_at = excluded.updated_at",
            rusqlite::params![
                ean,
                part_number,
                description,
                brand,
                price_brl_cents,
                stock_qty,
                chrono::Utc::now().timestamp()
            ],
        )?;
        drop(conn);
        self.product_cache.write().unwrap().clear();
        Ok(())
    }

    pub fn set_product_min_stock(&self, ean: &str, min_stock: f64) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE products SET min_stock=?1,updated_at=?2 WHERE ean=?3",
            rusqlite::params![min_stock, chrono::Utc::now().timestamp(), ean],
        )?;
        drop(conn);
        self.product_cache.write().unwrap().clear();
        Ok(())
    }

    pub fn create_purchase_order(&self, ean: &str, quantity: f64) -> rusqlite::Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute("INSERT INTO purchase_orders(product_ean,quantity,status,created_at) VALUES(?1,?2,'Pendente',?3)", rusqlite::params![ean,quantity,chrono::Utc::now().timestamp()])?;
        Ok(conn.last_insert_rowid())
    }

    pub fn record_sale(
        &self,
        items: &[SaleItemIn],
        payment: &PaymentIn,
        terminal_id: &str,
    ) -> rusqlite::Result<SaleOut> {
        self.record_sale_with_fiscal(items, payment, terminal_id, None)
    }

    pub fn record_sale_with_fiscal(
        &self,
        items: &[SaleItemIn],
        payment: &PaymentIn,
        terminal_id: &str,
        consumer_document: Option<&str>,
    ) -> rusqlite::Result<SaleOut> {
        let normalized_document = normalize_consumer_cpf(consumer_document)?;
        let consumer_document = normalized_document.as_deref();
        if items.is_empty() {
            return Err(rusqlite::Error::InvalidParameterName(
                "A venda precisa ter ao menos um item".into(),
            ));
        }
        if items
            .iter()
            .any(|item| !item.qty.is_finite() || item.qty <= 0.0 || item.price_brl_cents < 0)
        {
            return Err(rusqlite::Error::InvalidParameterName(
                "Item de venda inválido".into(),
            ));
        }
        let uuid = uuid::Uuid::new_v4().to_string();
        let total = items.iter().try_fold(0_i64, |sum, item| {
            let line = (item.price_brl_cents as f64 * item.qty).round();
            if !line.is_finite() || line >= i64::MAX as f64 {
                return Err(rusqlite::Error::InvalidParameterName(
                    "Valor de item excede o limite".into(),
                ));
            }
            sum.checked_add(line as i64).ok_or_else(|| {
                rusqlite::Error::InvalidParameterName("Total excede o limite".into())
            })
        })?;
        if payment.amount_brl_cents != total {
            return Err(rusqlite::Error::InvalidParameterName(
                "O valor recebido difere do total da venda".into(),
            ));
        }
        let created_at = chrono::Utc::now().timestamp();
        let items_json = serde_json::to_string(items).unwrap_or_else(|_| "[]".into());

        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        for item in items {
            let changed = tx.execute(
                "UPDATE products
                 SET stock_qty = stock_qty - ?1, updated_at = ?2
                 WHERE ean = ?3 AND active = 1 AND stock_qty >= ?1",
                rusqlite::params![item.qty, created_at, item.ean],
            )?;
            if changed != 1 {
                return Err(rusqlite::Error::InvalidParameterName(format!(
                    "Estoque insuficiente ou produto não encontrado: {}",
                    item.ean
                )));
            }
        }
        tx.execute(
            "INSERT INTO sales (uuid, total_brl_cents, payment_method, items_json, created_at, terminal_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![uuid, total, payment.method, items_json, created_at, terminal_id],
        )?;

        let payload = serde_json::json!({
            "uuid": uuid,
            "total_brl_cents": total,
            "payment_method": payment.method,
            "payment_extra": payment.extra,
            "items": items,
            "created_at": created_at,
            "terminal_id": terminal_id,
            "consumer_document": consumer_document,
        })
        .to_string();

        tx.execute(
            "INSERT INTO sync_outbox (uuid, entity, operation, payload, status, attempts, created_at)
             VALUES (?1, ?2, ?3, ?4, 'pending', 0, ?5)",
            rusqlite::params![
                uuid::Uuid::new_v4().to_string(),
                "sale",
                "insert",
                payload,
                created_at,
            ],
        )?;
        let outbox_id = tx.last_insert_rowid();
        tx.execute(
            "INSERT INTO fiscal_queue(sale_uuid,consumer_document,status,created_at,updated_at) VALUES(?1,?2,'pending_configuration',?3,?3)",
            rusqlite::params![uuid, consumer_document, created_at],
        )?;
        tx.commit()?;
        drop(conn);
        self.product_cache.write().unwrap().clear();

        Ok(SaleOut {
            sale_uuid: uuid,
            total_brl_cents: total,
            payment_method: payment.method.clone(),
            items_count: items.len(),
            outbox_id,
            created_at,
        })
    }

    pub fn enqueue_outbox(
        &self,
        entity: &str,
        operation: &str,
        payload: &str,
    ) -> rusqlite::Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO sync_outbox (uuid, entity, operation, payload, status, attempts, created_at)
             VALUES (?1, ?2, ?3, ?4, 'pending', 0, ?5)",
            rusqlite::params![
                uuid::Uuid::new_v4().to_string(),
                entity,
                operation,
                payload,
                chrono::Utc::now().timestamp()
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    fn outbox_from_row(row: &rusqlite::Row) -> rusqlite::Result<OutboxRow> {
        Ok(OutboxRow {
            id: row.get(0)?,
            uuid: row.get(1)?,
            entity: row.get(2)?,
            operation: row.get(3)?,
            payload: row.get(4)?,
            status: row.get(5)?,
            attempts: row.get(6)?,
            last_error: row.get(7)?,
            created_at: row.get(8)?,
        })
    }

    pub fn list_pending_outbox(&self, limit: i64) -> rusqlite::Result<Vec<OutboxRow>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, uuid, entity, operation, payload, status, attempts, last_error, created_at
             FROM sync_outbox WHERE status = 'pending' ORDER BY created_at ASC LIMIT ?1",
        )?;
        let rows = stmt.query_map([limit], Self::outbox_from_row)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn count_pending_outbox(&self) -> rusqlite::Result<usize> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT COUNT(*) FROM sync_outbox WHERE status = 'pending'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map(|count| count.max(0) as usize)
    }

    pub fn sync_conflicts(&self) -> rusqlite::Result<(usize, Option<String>)> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT COUNT(*),MAX(last_error) FROM sync_outbox WHERE status='pending' AND attempts>0",
            [],
            |row| Ok((row.get::<_, i64>(0)?.max(0) as usize, row.get(1)?)),
        )
    }

    pub fn mark_outbox_sent(&self, id: i64) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE sync_outbox SET status = 'sent', last_error = NULL WHERE id = ?1",
            [id],
        )?;
        Ok(())
    }

    pub fn mark_outbox_failed(&self, id: i64, error: &str) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE sync_outbox SET attempts = attempts + 1, last_error = ?1 WHERE id = ?2",
            rusqlite::params![error, id],
        )?;
        Ok(())
    }

    // ── Print Spool ──────────────────────────────────────────

    pub fn enqueue_print(&self, sale_uuid: &str, receipt_json: &str) -> rusqlite::Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO print_spool_queue (sale_uuid, receipt_json, status, attempts, created_at)
             VALUES (?1, ?2, 'pending', 0, ?3)",
            rusqlite::params![sale_uuid, receipt_json, chrono::Utc::now().timestamp()],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn list_pending_prints(&self) -> rusqlite::Result<Vec<PrintSpoolRow>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, sale_uuid, receipt_json, status, attempts, last_error, created_at
             FROM print_spool_queue WHERE status = 'pending' ORDER BY created_at ASC LIMIT 20",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(PrintSpoolRow {
                id: row.get(0)?,
                sale_uuid: row.get(1)?,
                receipt_json: row.get(2)?,
                status: row.get(3)?,
                attempts: row.get(4)?,
                last_error: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        rows.collect()
    }

    pub fn mark_print_sent(&self, id: i64) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE print_spool_queue SET status = 'sent', last_error = NULL WHERE id = ?1",
            [id],
        )?;
        Ok(())
    }

    pub fn mark_print_failed(&self, id: i64, error: &str) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE print_spool_queue SET attempts = attempts + 1, last_error = ?1, status = 'failed' WHERE id = ?2 AND attempts >= 4",
            rusqlite::params![error, id],
        )?;
        conn.execute(
            "UPDATE print_spool_queue SET attempts = attempts + 1, last_error = ?1 WHERE id = ?2 AND attempts < 4",
            rusqlite::params![error, id],
        )?;
        Ok(())
    }

    // ── Cash Movements ───────────────────────────────────────

    pub fn record_cash_movement(
        &self,
        session_id: i64,
        movement_type: &str,
        amount_brl_cents: i64,
        description: Option<&str>,
        operator: Option<&str>,
    ) -> rusqlite::Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO cash_movements (session_id, type, amount_brl_cents, description, operator, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                session_id,
                movement_type,
                amount_brl_cents,
                description,
                operator,
                chrono::Utc::now().timestamp()
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn cash_movements_for_session(
        &self,
        session_id: i64,
    ) -> rusqlite::Result<Vec<CashMovement>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, session_id, type, amount_brl_cents, description, operator, created_at
             FROM cash_movements WHERE session_id = ?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map([session_id], |row| {
            Ok(CashMovement {
                id: row.get(0)?,
                session_id: row.get(1)?,
                movement_type: row.get(2)?,
                amount_brl_cents: row.get(3)?,
                description: row.get(4)?,
                operator: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        rows.collect()
    }

    pub fn sales_by_method_since(&self, since: i64) -> rusqlite::Result<serde_json::Value> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT payment_method, COALESCE(SUM(total_brl_cents), 0), COUNT(*)
             FROM sales WHERE created_at >= ?1 GROUP BY payment_method",
        )?;
        let rows = stmt.query_map([since], |row| {
            let method: String = row.get(0)?;
            let total: i64 = row.get(1)?;
            let count: i64 = row.get(2)?;
            Ok(serde_json::json!({
                "method": method,
                "total_brl_cents": total,
                "count": count
            }))
        })?;
        let methods: Vec<serde_json::Value> = rows.filter_map(|r| r.ok()).collect();
        Ok(serde_json::json!({ "methods": methods }))
    }

    // ── Cash Status (existing, with movements) ──────────────

    fn sales_since_conn(conn: &Connection, since: i64) -> rusqlite::Result<i64> {
        conn.query_row(
            "SELECT COALESCE(SUM(total_brl_cents), 0) FROM sales WHERE created_at >= ?1 AND payment_method='cash'",
            [since],
            |r| r.get(0),
        )
    }

    fn cash_adjustments_conn(conn: &Connection, session_id: i64) -> rusqlite::Result<i64> {
        conn.query_row(
            "SELECT COALESCE(SUM(CASE WHEN type='suprimento' THEN amount_brl_cents WHEN type='sangria' THEN -amount_brl_cents ELSE 0 END),0) FROM cash_movements WHERE session_id=?1",
            [session_id], |row| row.get(0),
        )
    }

    fn cash_status_conn(conn: &Connection) -> rusqlite::Result<CashStatus> {
        let row = conn.query_row(
            "SELECT id, opened_at, opening_brl_cents, closed_at, closing_brl_cents,
                    expected_brl_cents, difference_brl_cents
             FROM cash_sessions ORDER BY id DESC LIMIT 1",
            [],
            |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, Option<i64>>(3)?,
                    r.get::<_, Option<i64>>(4)?,
                    r.get::<_, Option<i64>>(5)?,
                    r.get::<_, Option<i64>>(6)?,
                ))
            },
        );
        let st = match row {
            Ok(st) => st,
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                return Ok(CashStatus {
                    open: false,
                    session_id: None,
                    opened_at: None,
                    opening_brl_cents: 0,
                    sales_brl_cents: 0,
                    expected_brl_cents: 0,
                    closed_at: None,
                    closing_brl_cents: None,
                    difference_brl_cents: 0,
                })
            }
            Err(e) => return Err(e),
        };
        let (id, opened_at, opening, closed_at, closing, expected, diff) = st;
        if closed_at.is_some() {
            return Ok(CashStatus {
                open: false,
                session_id: Some(id),
                opened_at: Some(opened_at),
                opening_brl_cents: opening,
                sales_brl_cents: 0,
                expected_brl_cents: expected.unwrap_or(0),
                closed_at,
                closing_brl_cents: closing,
                difference_brl_cents: diff.unwrap_or(0),
            });
        }
        let sales = Self::sales_since_conn(conn, opened_at)?;
        let adjustments = Self::cash_adjustments_conn(conn, id)?;
        Ok(CashStatus {
            open: true,
            session_id: Some(id),
            opened_at: Some(opened_at),
            opening_brl_cents: opening,
            sales_brl_cents: sales,
            expected_brl_cents: opening + sales + adjustments,
            closed_at: None,
            closing_brl_cents: None,
            difference_brl_cents: 0,
        })
    }

    pub fn cash_status(&self) -> rusqlite::Result<CashStatus> {
        let conn = self.conn.lock().unwrap();
        Self::cash_status_conn(&conn)
    }

    pub fn cash_open(&self, opening_brl_cents: i64) -> rusqlite::Result<CashStatus> {
        let conn = self.conn.lock().unwrap();
        let current = Self::cash_status_conn(&conn)?;
        if current.open {
            return Ok(current);
        }
        conn.execute(
            "INSERT INTO cash_sessions (opened_at, opening_brl_cents) VALUES (?1, ?2)",
            rusqlite::params![chrono::Utc::now().timestamp(), opening_brl_cents],
        )?;
        Self::cash_status_conn(&conn)
    }

    pub fn cash_close(&self, closing_brl_cents: i64) -> rusqlite::Result<CashStatus> {
        let conn = self.conn.lock().unwrap();
        let current = Self::cash_status_conn(&conn)?;
        if !current.open || current.session_id.is_none() {
            return Ok(current);
        }
        let session_id = current.session_id.unwrap();
        let opened_at = current.opened_at.unwrap_or(0);
        let sales = Self::sales_since_conn(&conn, opened_at)?;
        let adjustments = Self::cash_adjustments_conn(&conn, session_id)?;
        let expected = current.opening_brl_cents + sales + adjustments;
        let diff = closing_brl_cents - expected;
        conn.execute(
            "UPDATE cash_sessions SET closed_at = ?1, closing_brl_cents = ?2,
                    expected_brl_cents = ?3, difference_brl_cents = ?4 WHERE id = ?5",
            rusqlite::params![
                chrono::Utc::now().timestamp(),
                closing_brl_cents,
                expected,
                diff,
                session_id
            ],
        )?;
        Self::cash_status_conn(&conn)
    }

    // ── Customers (Fidelidade) ──────────────────────────────────

    pub fn upsert_customer(
        &self,
        name: &str,
        cpf_cnpj: Option<&str>,
        phone: Option<&str>,
        email: Option<&str>,
    ) -> rusqlite::Result<Customer> {
        let conn = self.conn.lock().unwrap();
        if let Some(doc) = cpf_cnpj {
            let existing = conn.query_row(
                "SELECT id, name, cpf_cnpj, phone, email, points, total_spent_brl_cents, created_at
                 FROM customers WHERE cpf_cnpj = ?1",
                [doc],
                |row| Self::customer_from_row(row),
            );
            if let Ok(c) = existing {
                return Ok(c);
            }
        }
        conn.execute(
            "INSERT INTO customers (name, cpf_cnpj, phone, email, points, total_spent_brl_cents, created_at)
             VALUES (?1, ?2, ?3, ?4, 0, 0, ?5)",
            rusqlite::params![name, cpf_cnpj, phone, email, chrono::Utc::now().timestamp()],
        )?;
        let id = conn.last_insert_rowid();
        let c = conn.query_row(
            "SELECT id, name, cpf_cnpj, phone, email, points, total_spent_brl_cents, created_at
             FROM customers WHERE id = ?1",
            [id],
            |row| Self::customer_from_row(row),
        )?;
        Ok(c)
    }

    pub fn update_customer(
        &self,
        id: i64,
        name: &str,
        cpf_cnpj: Option<&str>,
        phone: Option<&str>,
        email: Option<&str>,
    ) -> rusqlite::Result<Customer> {
        let conn = self.conn.lock().unwrap();
        let changed = conn.execute(
            "UPDATE customers SET name=?1,cpf_cnpj=?2,phone=?3,email=?4 WHERE id=?5",
            rusqlite::params![name, cpf_cnpj, phone, email, id],
        )?;
        if changed != 1 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        conn.query_row(
            "SELECT id,name,cpf_cnpj,phone,email,points,total_spent_brl_cents,created_at FROM customers WHERE id=?1",
            [id], Self::customer_from_row,
        )
    }

    pub fn search_customers(&self, query: &str) -> rusqlite::Result<Vec<Customer>> {
        let conn = self.conn.lock().unwrap();
        let like = format!("%{query}%");
        let mut stmt = conn.prepare(
            "SELECT id, name, cpf_cnpj, phone, email, points, total_spent_brl_cents, created_at
             FROM customers WHERE name LIKE ?1 OR cpf_cnpj LIKE ?1 OR phone LIKE ?1
             ORDER BY name LIMIT 20",
        )?;
        let rows = stmt.query_map([&like], Self::customer_from_row)?;
        rows.collect()
    }

    pub fn list_customers(&self, limit: i64) -> rusqlite::Result<Vec<Customer>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, cpf_cnpj, phone, email, points, total_spent_brl_cents, created_at
             FROM customers ORDER BY points DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map([limit], Self::customer_from_row)?;
        rows.collect()
    }

    pub fn add_points(
        &self,
        customer_id: i64,
        points: i64,
        sale_uuid: Option<&str>,
        description: Option<&str>,
    ) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE customers SET points = points + ?1 WHERE id = ?2",
            rusqlite::params![points, customer_id],
        )?;
        conn.execute(
            "INSERT INTO points_history (customer_id, points, sale_uuid, description, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                customer_id,
                points,
                sale_uuid,
                description,
                chrono::Utc::now().timestamp()
            ],
        )?;
        Ok(())
    }

    pub fn redeem_points(
        &self,
        customer_id: i64,
        points: i64,
        description: Option<&str>,
    ) -> rusqlite::Result<i64> {
        let conn = self.conn.lock().unwrap();
        let current: i64 = conn.query_row(
            "SELECT points FROM customers WHERE id = ?1",
            [customer_id],
            |r| r.get(0),
        )?;
        if current < points {
            return Err(rusqlite::Error::InvalidParameterName(
                "Saldo de pontos insuficiente".into(),
            ));
        }
        conn.execute(
            "UPDATE customers SET points = points - ?1 WHERE id = ?2",
            rusqlite::params![points, customer_id],
        )?;
        conn.execute(
            "INSERT INTO points_history (customer_id, points, sale_uuid, description, created_at)
             VALUES (?1, ?2, NULL, ?3, ?4)",
            rusqlite::params![
                customer_id,
                -points,
                description,
                chrono::Utc::now().timestamp()
            ],
        )?;
        let remaining: i64 = conn.query_row(
            "SELECT points FROM customers WHERE id = ?1",
            [customer_id],
            |r| r.get(0),
        )?;
        Ok(remaining)
    }

    fn customer_from_row(row: &rusqlite::Row) -> rusqlite::Result<Customer> {
        Ok(Customer {
            id: row.get(0)?,
            name: row.get(1)?,
            cpf_cnpj: row.get(2)?,
            phone: row.get(3)?,
            email: row.get(4)?,
            points: row.get(5)?,
            total_spent_brl_cents: row.get(6)?,
            created_at: row.get(7)?,
        })
    }

    pub fn list_employees(&self) -> rusqlite::Result<Vec<Employee>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare_cached(
            "SELECT id,name,role,status FROM employees ORDER BY status DESC,name COLLATE NOCASE",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Employee {
                id: row.get(0)?,
                name: row.get(1)?,
                role: row.get(2)?,
                status: row.get(3)?,
            })
        })?;
        let result = rows.collect();
        result
    }

    pub fn list_time_entries(&self, limit: i64) -> rusqlite::Result<Vec<TimeEntry>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare_cached(
            "SELECT t.id,t.employee_id,e.name,t.event_type,t.note,t.occurred_at
             FROM time_entries t JOIN employees e ON e.id=t.employee_id
             ORDER BY t.occurred_at DESC,t.id DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map([limit.clamp(1, 1000)], |row| {
            Ok(TimeEntry {
                id: row.get(0)?,
                employee_id: row.get(1)?,
                employee_name: row.get(2)?,
                event_type: row.get(3)?,
                note: row.get(4)?,
                occurred_at: row.get(5)?,
            })
        })?;
        rows.collect()
    }

    pub fn record_time_entry(
        &self,
        employee_id: i64,
        event_type: &str,
        note: Option<&str>,
    ) -> rusqlite::Result<TimeEntry> {
        let conn = self.conn.lock().unwrap();
        let employee_name: String = conn.query_row(
            "SELECT name FROM employees WHERE id=?1 AND status='Ativo'",
            [employee_id],
            |row| row.get(0),
        )?;
        let occurred_at = chrono::Utc::now().timestamp();
        conn.execute(
            "INSERT INTO time_entries(employee_id,event_type,note,occurred_at) VALUES(?1,?2,?3,?4)",
            rusqlite::params![employee_id, event_type, note, occurred_at],
        )?;
        Ok(TimeEntry {
            id: conn.last_insert_rowid(),
            employee_id,
            employee_name,
            event_type: event_type.into(),
            note: note.map(str::to_owned),
            occurred_at,
        })
    }

    pub fn save_employee(
        &self,
        name: &str,
        role: &str,
        status: &str,
    ) -> rusqlite::Result<Employee> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO employees(name,role,status,updated_at) VALUES(?1,?2,?3,?4)",
            rusqlite::params![name, role, status, chrono::Utc::now().timestamp()],
        )?;
        Ok(Employee {
            id: conn.last_insert_rowid(),
            name: name.into(),
            role: role.into(),
            status: status.into(),
        })
    }

    pub fn set_employee_status(&self, id: i64, status: &str) -> rusqlite::Result<bool> {
        let conn = self.conn.lock().unwrap();
        Ok(conn.execute(
            "UPDATE employees SET status=?1,updated_at=?2 WHERE id=?3",
            rusqlite::params![status, chrono::Utc::now().timestamp(), id],
        )? == 1)
    }

    pub fn update_employee(
        &self,
        id: i64,
        name: &str,
        role: &str,
        status: &str,
    ) -> rusqlite::Result<Option<Employee>> {
        let conn = self.conn.lock().unwrap();
        let changed = conn.execute(
            "UPDATE employees SET name=?1,role=?2,status=?3,updated_at=?4 WHERE id=?5",
            rusqlite::params![name, role, status, chrono::Utc::now().timestamp(), id],
        )?;
        Ok((changed == 1).then(|| Employee {
            id,
            name: name.into(),
            role: role.into(),
            status: status.into(),
        }))
    }

    pub fn list_expenses(&self) -> rusqlite::Result<Vec<Expense>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare_cached(
            "SELECT id,description,category,amount_brl_cents,due_date,status FROM expenses ORDER BY due_date,id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Expense {
                id: row.get(0)?,
                description: row.get(1)?,
                category: row.get(2)?,
                amount_brl_cents: row.get(3)?,
                due_date: row.get(4)?,
                status: row.get(5)?,
            })
        })?;
        let result = rows.collect();
        result
    }

    pub fn save_expense(
        &self,
        description: &str,
        category: &str,
        amount: i64,
        due_date: &str,
        status: &str,
    ) -> rusqlite::Result<Expense> {
        let conn = self.conn.lock().unwrap();
        conn.execute("INSERT INTO expenses(description,category,amount_brl_cents,due_date,status,updated_at) VALUES(?1,?2,?3,?4,?5,?6)", rusqlite::params![description,category,amount,due_date,status,chrono::Utc::now().timestamp()])?;
        Ok(Expense {
            id: conn.last_insert_rowid(),
            description: description.into(),
            category: category.into(),
            amount_brl_cents: amount,
            due_date: due_date.into(),
            status: status.into(),
        })
    }

    pub fn set_expense_status(&self, id: i64, status: &str) -> rusqlite::Result<bool> {
        let conn = self.conn.lock().unwrap();
        Ok(conn.execute(
            "UPDATE expenses SET status=?1,updated_at=?2 WHERE id=?3",
            rusqlite::params![status, chrono::Utc::now().timestamp(), id],
        )? == 1)
    }

    pub fn update_expense(
        &self,
        id: i64,
        description: &str,
        category: &str,
        amount: i64,
        due_date: &str,
        status: &str,
    ) -> rusqlite::Result<Option<Expense>> {
        let conn = self.conn.lock().unwrap();
        let changed = conn.execute("UPDATE expenses SET description=?1,category=?2,amount_brl_cents=?3,due_date=?4,status=?5,updated_at=?6 WHERE id=?7", rusqlite::params![description, category, amount, due_date, status, chrono::Utc::now().timestamp(), id])?;
        Ok((changed == 1).then(|| Expense {
            id,
            description: description.into(),
            category: category.into(),
            amount_brl_cents: amount,
            due_date: due_date.into(),
            status: status.into(),
        }))
    }

    pub fn list_suppliers(&self) -> rusqlite::Result<Vec<Supplier>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare_cached(
            "SELECT id,name,document,phone,status FROM suppliers ORDER BY name COLLATE NOCASE",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Supplier {
                id: row.get(0)?,
                name: row.get(1)?,
                document: row.get(2)?,
                phone: row.get(3)?,
                status: row.get(4)?,
            })
        })?;
        let result = rows.collect();
        result
    }

    pub fn save_supplier(
        &self,
        name: &str,
        document: Option<&str>,
        phone: Option<&str>,
        status: &str,
    ) -> rusqlite::Result<Supplier> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO suppliers(name,document,phone,status,updated_at) VALUES(?1,?2,?3,?4,?5)",
            rusqlite::params![
                name,
                document,
                phone,
                status,
                chrono::Utc::now().timestamp()
            ],
        )?;
        Ok(Supplier {
            id: conn.last_insert_rowid(),
            name: name.into(),
            document: document.map(Into::into),
            phone: phone.map(Into::into),
            status: status.into(),
        })
    }

    pub fn set_supplier_status(&self, id: i64, status: &str) -> rusqlite::Result<bool> {
        let conn = self.conn.lock().unwrap();
        Ok(conn.execute(
            "UPDATE suppliers SET status=?1,updated_at=?2 WHERE id=?3",
            rusqlite::params![status, chrono::Utc::now().timestamp(), id],
        )? == 1)
    }

    pub fn update_supplier(
        &self,
        id: i64,
        name: &str,
        document: Option<&str>,
        phone: Option<&str>,
        status: &str,
    ) -> rusqlite::Result<Option<Supplier>> {
        let conn = self.conn.lock().unwrap();
        let changed = conn.execute(
            "UPDATE suppliers SET name=?1,document=?2,phone=?3,status=?4,updated_at=?5 WHERE id=?6",
            rusqlite::params![
                name,
                document,
                phone,
                status,
                chrono::Utc::now().timestamp(),
                id
            ],
        )?;
        Ok((changed == 1).then(|| Supplier {
            id,
            name: name.into(),
            document: document.map(Into::into),
            phone: phone.map(Into::into),
            status: status.into(),
        }))
    }

    pub fn list_promotions(&self, include_inactive: bool) -> rusqlite::Result<Vec<Promotion>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare_cached(
            "SELECT id,title,subtitle,price_label,active FROM promotions WHERE (?1=1 OR active=1) ORDER BY updated_at DESC,id",
        )?;
        let rows = stmt.query_map([include_inactive as i64], |row| {
            Ok(Promotion {
                id: row.get(0)?,
                title: row.get(1)?,
                subtitle: row.get(2)?,
                price_label: row.get(3)?,
                active: row.get::<_, i64>(4)? != 0,
            })
        })?;
        let result = rows.collect();
        result
    }

    pub fn save_promotion(
        &self,
        title: &str,
        subtitle: Option<&str>,
        price_label: &str,
        active: bool,
    ) -> rusqlite::Result<Promotion> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO promotions(title,subtitle,price_label,active,updated_at) VALUES(?1,?2,?3,?4,?5)",
            rusqlite::params![title, subtitle, price_label, active as i64, chrono::Utc::now().timestamp()],
        )?;
        let promotion = Promotion {
            id: conn.last_insert_rowid(),
            title: title.into(),
            subtitle: subtitle.map(Into::into),
            price_label: price_label.into(),
            active,
        };
        let payload = serde_json::json!({"local_id":promotion.id,"title":promotion.title,"subtitle":promotion.subtitle,"price_label":promotion.price_label,"active":promotion.active}).to_string();
        conn.execute("INSERT INTO sync_outbox(uuid,entity,operation,payload,status,attempts,created_at) VALUES(?1,'promotion','upsert',?2,'pending',0,?3)", rusqlite::params![uuid::Uuid::new_v4().to_string(), payload, chrono::Utc::now().timestamp()])?;
        Ok(promotion)
    }

    pub fn set_promotion_active(&self, id: i64, active: bool) -> rusqlite::Result<bool> {
        let conn = self.conn.lock().unwrap();
        let changed = conn.execute(
            "UPDATE promotions SET active=?1,updated_at=?2 WHERE id=?3",
            rusqlite::params![active as i64, chrono::Utc::now().timestamp(), id],
        )? == 1;
        if changed {
            let payload = serde_json::json!({"local_id":id,"active":active}).to_string();
            conn.execute("INSERT INTO sync_outbox(uuid,entity,operation,payload,status,attempts,created_at) VALUES(?1,'promotion','upsert',?2,'pending',0,?3)", rusqlite::params![uuid::Uuid::new_v4().to_string(), payload, chrono::Utc::now().timestamp()])?;
        }
        Ok(changed)
    }

    pub fn update_promotion(
        &self,
        id: i64,
        title: &str,
        subtitle: Option<&str>,
        price_label: &str,
        active: bool,
    ) -> rusqlite::Result<Option<Promotion>> {
        let conn = self.conn.lock().unwrap();
        let changed = conn.execute(
            "UPDATE promotions SET title=?1,subtitle=?2,price_label=?3,active=?4,updated_at=?5 WHERE id=?6",
            rusqlite::params![title, subtitle, price_label, active as i64, chrono::Utc::now().timestamp(), id],
        )?;
        let promotion = (changed == 1).then(|| Promotion {
            id,
            title: title.into(),
            subtitle: subtitle.map(Into::into),
            price_label: price_label.into(),
            active,
        });
        if let Some(promotion) = &promotion {
            let payload = serde_json::json!({"local_id":promotion.id,"title":promotion.title,"subtitle":promotion.subtitle,"price_label":promotion.price_label,"active":promotion.active}).to_string();
            conn.execute("INSERT INTO sync_outbox(uuid,entity,operation,payload,status,attempts,created_at) VALUES(?1,'promotion','upsert',?2,'pending',0,?3)", rusqlite::params![uuid::Uuid::new_v4().to_string(), payload, chrono::Utc::now().timestamp()])?;
        }
        Ok(promotion)
    }

    pub fn dashboard_summary(&self) -> rusqlite::Result<DashboardSummary> {
        let conn = self.conn.lock().unwrap();
        let today = chrono::Utc::now()
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp();
        let sales_today_brl_cents = conn.query_row(
            "SELECT COALESCE(SUM(total_brl_cents),0) FROM sales WHERE created_at>=?1",
            [today],
            |row| row.get(0),
        )?;
        let items_today = conn.query_row(
            "SELECT COALESCE(SUM(json_array_length(items_json)),0) FROM sales WHERE created_at>=?1",
            [today],
            |row| row.get(0),
        )?;
        let new_customers_today = conn.query_row(
            "SELECT COUNT(*) FROM customers WHERE created_at>=?1",
            [today],
            |row| row.get(0),
        )?;
        let low_stock_count = conn.query_row(
            "SELECT COUNT(*) FROM products WHERE stock_qty<min_stock",
            [],
            |row| row.get(0),
        )?;
        let mut daily_sales = Vec::with_capacity(7);
        for days_ago in (0..7).rev() {
            let start = today - days_ago * 86_400;
            let end = start + 86_400;
            daily_sales.push(conn.query_row(
                "SELECT COALESCE(SUM(total_brl_cents),0) FROM sales WHERE created_at>=?1 AND created_at<?2",
                rusqlite::params![start, end],
                |row| row.get(0),
            )?);
        }
        Ok(DashboardSummary {
            sales_today_brl_cents,
            items_today,
            new_customers_today,
            low_stock_count,
            daily_sales,
        })
    }

    pub fn current_payment_totals(&self) -> rusqlite::Result<PaymentTotals> {
        let conn = self.conn.lock().unwrap();
        let opened_at = conn
            .query_row(
                "SELECT opened_at FROM cash_sessions WHERE closed_at IS NULL ORDER BY id DESC LIMIT 1",
                [],
                |row| row.get::<_, i64>(0),
            )
            .optional()?;
        let Some(opened_at) = opened_at else {
            return Ok(PaymentTotals::default());
        };
        let mut totals = PaymentTotals::default();
        let mut stmt = conn.prepare_cached(
            "SELECT payment_method,COALESCE(SUM(total_brl_cents),0) FROM sales WHERE created_at>=?1 GROUP BY payment_method",
        )?;
        let rows = stmt.query_map([opened_at], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        for row in rows {
            let (method, value) = row?;
            match method.as_str() {
                "cash" => totals.cash_brl_cents += value,
                "credit" | "credit_card" | "debit" => totals.card_brl_cents += value,
                "pix" => totals.pix_brl_cents += value,
                _ => totals.other_brl_cents += value,
            }
        }
        Ok(totals)
    }

    pub fn sales_report(
        &self,
        from: i64,
        to: i64,
        payment_method: Option<&str>,
        terminal_id: Option<&str>,
        product_query: Option<&str>,
    ) -> rusqlite::Result<Vec<SalesReportRow>> {
        let conn = self.conn.lock().unwrap();
        let product_like = product_query.map(|value| format!("%{value}%"));
        let mut stmt = conn.prepare_cached(
            "SELECT s.uuid,s.total_brl_cents,s.payment_method,s.terminal_id,
                    json_array_length(s.items_json),s.created_at
             FROM sales s
             WHERE s.created_at>=?1 AND s.created_at<?2
               AND (?3 IS NULL OR s.payment_method=?3)
               AND (?4 IS NULL OR s.terminal_id=?4)
               AND (?5 IS NULL OR EXISTS(
                    SELECT 1 FROM json_each(s.items_json) item
                    WHERE json_extract(item.value,'$.ean') LIKE ?5))
             ORDER BY s.created_at DESC,s.id DESC LIMIT 5000",
        )?;
        let rows = stmt.query_map(
            rusqlite::params![from, to, payment_method, terminal_id, product_like],
            |row| {
                Ok(SalesReportRow {
                    uuid: row.get(0)?,
                    total_brl_cents: row.get(1)?,
                    payment_method: row.get(2)?,
                    terminal_id: row.get(3)?,
                    items_count: row.get(4)?,
                    created_at: row.get(5)?,
                })
            },
        )?;
        let result = rows.collect();
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temporary_db(name: &str) -> (Db, PathBuf) {
        let path =
            std::env::temp_dir().join(format!("commercectrl-{name}-{}.db", uuid::Uuid::new_v4()));
        (Db::open(&path).expect("abrir banco temporário"), path)
    }

    fn remove_database(path: &Path) {
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(format!("{}-wal", path.display()));
        let _ = std::fs::remove_file(format!("{}-shm", path.display()));
    }

    #[test]
    fn cash_lifecycle_does_not_relock_connection() {
        let (db, path) = temporary_db("cash");
        let opened = db.cash_open(10_000).expect("abrir caixa");
        assert!(opened.open);
        assert_eq!(opened.opening_brl_cents, 10_000);

        let status = db.cash_status().expect("consultar caixa");
        assert!(status.open);

        let session_id = status.session_id.unwrap();
        db.record_cash_movement(session_id, "sangria", 1_000, None, None)
            .unwrap();
        db.record_cash_movement(session_id, "suprimento", 500, None, None)
            .unwrap();
        assert_eq!(db.cash_status().unwrap().expected_brl_cents, 9_500);

        let closed = db.cash_close(9_500).expect("fechar caixa");
        assert!(!closed.open);
        assert_eq!(closed.difference_brl_cents, 0);
        drop(db);
        remove_database(&path);
    }

    #[test]
    fn cpf_is_validated_before_sale_and_normalized_in_fiscal_queue() {
        let db = Db::open(Path::new(":memory:")).unwrap();
        db.upsert_product("cpf", "cpf", "Produto", None, 100, 5.0)
            .unwrap();
        let items = [SaleItemIn {
            ean: "cpf".into(),
            qty: 1.0,
            price_brl_cents: 100,
        }];
        let payment = PaymentIn {
            method: "cash".into(),
            amount_brl_cents: 100,
            extra: None,
        };
        for cpf in ["11111111111", "52998224724", "texto52998224725", "123"] {
            assert!(db
                .record_sale_with_fiscal(&items, &payment, "test", Some(cpf))
                .is_err());
        }
        assert_eq!(db.count_pending_outbox().unwrap(), 0);
        assert_eq!(db.search_by_ean("cpf").unwrap().unwrap().stock_qty, 5.0);
        let sale = db
            .record_sale_with_fiscal(&items, &payment, "test", Some("529.982.247-25"))
            .unwrap();
        let conn = db.conn.lock().unwrap();
        let stored: String = conn
            .query_row(
                "SELECT consumer_document FROM fiscal_queue WHERE sale_uuid=?1",
                [sale.sale_uuid],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(stored, "52998224725");
        assert_eq!(normalize_consumer_cpf(Some(" ")).unwrap(), None);
    }

    #[test]
    fn failed_migration_rolls_back_schema_data_and_version_then_can_retry() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE inventory (id INTEGER PRIMARY KEY); INSERT INTO inventory VALUES(1); PRAGMA user_version=1;").unwrap();
        let broken = [("2", "ALTER TABLE inventory ADD COLUMN note TEXT; UPDATE inventory SET note='alterado'; INSERT INTO missing_table VALUES(1);")];
        assert!(Db::apply_migrations(&mut conn, &broken).is_err());
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 1);
        let columns: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('inventory')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(columns, 1);
        let repaired = [(
            "2",
            "ALTER TABLE inventory ADD COLUMN note TEXT; UPDATE inventory SET note='preservado';",
        )];
        Db::apply_migrations(&mut conn, &repaired).unwrap();
        Db::apply_migrations(&mut conn, &repaired).unwrap();
        let note: String = conn
            .query_row("SELECT note FROM inventory WHERE id=1", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(note, "preservado");
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 2);
    }

    #[test]
    fn archived_item_rolls_back_entire_sale_including_fiscal_queue() {
        let db = Db::open(Path::new(":memory:")).unwrap();
        db.upsert_product("active", "active", "Ativo", None, 100, 5.0)
            .unwrap();
        db.upsert_product("archived", "archived", "Arquivado", None, 100, 5.0)
            .unwrap();
        db.set_product_active("archived", false).unwrap();
        let items = ["active", "archived"].map(|ean| SaleItemIn {
            ean: ean.into(),
            qty: 1.0,
            price_brl_cents: 100,
        });
        let payment = PaymentIn {
            method: "cash".into(),
            amount_brl_cents: 200,
            extra: None,
        };
        assert!(db
            .record_sale_with_fiscal(&items, &payment, "test", None)
            .is_err());
        assert_eq!(db.search_by_ean("active").unwrap().unwrap().stock_qty, 5.0);
        assert_eq!(db.count_pending_outbox().unwrap(), 0);
        let conn = db.conn.lock().unwrap();
        for table in ["sales", "fiscal_queue"] {
            let count: i64 = conn
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .unwrap();
            assert_eq!(count, 0, "{table} deve ficar intacta");
        }
    }

    #[test]
    fn invalid_numeric_values_do_not_panic_or_mutate_stock() {
        let db = Db::open(Path::new(":memory:")).unwrap();
        db.upsert_product("numeric", "numeric", "Produto", None, 100, 5.0)
            .unwrap();
        let payment = PaymentIn {
            method: "cash".into(),
            amount_brl_cents: 0,
            extra: None,
        };
        for qty in [
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            -1.0,
            0.0,
            f64::MAX,
        ] {
            let items = [SaleItemIn {
                ean: "numeric".into(),
                qty,
                price_brl_cents: 100,
            }];
            assert!(db.record_sale(&items, &payment, "test").is_err());
        }
        let items = vec![
            SaleItemIn {
                ean: "numeric".into(),
                qty: 1.0,
                price_brl_cents: i64::MAX / 2
            };
            3
        ];
        assert!(db.record_sale(&items, &payment, "test").is_err());
        assert_eq!(db.search_by_ean("numeric").unwrap().unwrap().stock_qty, 5.0);
        assert_eq!(db.count_pending_outbox().unwrap(), 0);
    }

    #[test]
    fn sale_stock_and_outbox_commit_atomically() {
        let (db, path) = temporary_db("sale");
        db.upsert_product("7891", "SKU-1", "Produto", None, 750, 10.0)
            .expect("criar produto");

        let items = vec![SaleItemIn {
            ean: "7891".into(),
            qty: 2.0,
            price_brl_cents: 750,
        }];
        let payment = PaymentIn {
            method: "cash".into(),
            amount_brl_cents: 1_500,
            extra: None,
        };
        db.record_sale(&items, &payment, "terminal-1")
            .expect("gravar venda");

        assert_eq!(
            db.search_by_ean("7891")
                .expect("buscar produto")
                .expect("produto existe")
                .stock_qty,
            8.0
        );
        assert_eq!(db.list_pending_outbox(10).expect("outbox").len(), 1);
        assert_eq!(db.count_pending_outbox().expect("contador outbox"), 1);

        let unavailable = vec![SaleItemIn {
            qty: 20.0,
            ..items[0].clone()
        }];
        assert!(db
            .record_sale(
                &unavailable,
                &PaymentIn {
                    amount_brl_cents: 15_000,
                    ..payment
                },
                "terminal-1"
            )
            .is_err());
        assert_eq!(
            db.search_by_ean("7891")
                .expect("buscar após rollback")
                .expect("produto existe")
                .stock_qty,
            8.0
        );
        assert_eq!(db.list_pending_outbox(10).expect("outbox").len(), 1);
        assert_eq!(db.count_pending_outbox().expect("contador outbox"), 1);
        drop(db);
        remove_database(&path);
    }

    #[test]
    fn product_cache_is_invalidated_after_stock_change() {
        let (db, path) = temporary_db("cache");
        db.upsert_product("7892", "SKU-2", "Outro produto", None, 500, 5.0)
            .expect("criar produto");
        assert_eq!(
            db.search_by_ean("7892")
                .expect("popular cache")
                .expect("produto existe")
                .stock_qty,
            5.0
        );
        db.adjust_stock("7892", -1.0).expect("ajustar estoque");
        assert_eq!(
            db.search_by_ean("7892")
                .expect("recarregar produto")
                .expect("produto existe")
                .stock_qty,
            4.0
        );
        drop(db);
        remove_database(&path);
    }

    #[test]
    fn operational_seed_and_create_flows_are_persisted() {
        let (db, path) = temporary_db("operations");
        assert!(!db.list_products(100, 0).unwrap().is_empty());
        assert!(!db.list_employees().unwrap().is_empty());
        assert!(!db.list_expenses().unwrap().is_empty());
        assert!(!db.list_suppliers().unwrap().is_empty());
        assert!(!db.list_promotions(false).unwrap().is_empty());
        assert!(!db.list_customers(100).unwrap().is_empty());

        let employee = db.save_employee("Teste", "Caixa", "Ativo").unwrap();
        assert!(db
            .list_employees()
            .unwrap()
            .iter()
            .any(|x| x.id == employee.id));
        let expense = db
            .save_expense("Internet", "Fixa", 9990, "2030-01-01", "Pendente")
            .unwrap();
        assert!(db
            .list_expenses()
            .unwrap()
            .iter()
            .any(|x| x.id == expense.id));
        let supplier = db
            .save_supplier("Fornecedor Teste", Some("123"), Some("456"), "Ativo")
            .unwrap();
        assert!(db
            .list_suppliers()
            .unwrap()
            .iter()
            .any(|x| x.id == supplier.id));
        let promotion = db
            .save_promotion("Oferta", Some("Teste"), "R$ 9,90", true)
            .unwrap();
        assert!(db
            .list_promotions(true)
            .unwrap()
            .iter()
            .any(|x| x.id == promotion.id));
        assert!(db.set_employee_status(employee.id, "Inativo").unwrap());
        assert!(db.set_expense_status(expense.id, "Pago").unwrap());
        assert!(db.set_supplier_status(supplier.id, "Inativo").unwrap());
        assert_eq!(
            db.update_employee(employee.id, "Ana Editada", "Gerente", "Ativo")
                .unwrap()
                .unwrap()
                .role,
            "Gerente"
        );
        assert_eq!(
            db.update_expense(
                expense.id,
                "Internet editada",
                "Fixa",
                10990,
                "2030-02-01",
                "Pendente"
            )
            .unwrap()
            .unwrap()
            .amount_brl_cents,
            10990
        );
        assert_eq!(
            db.update_supplier(
                supplier.id,
                "Fornecedor Editado",
                Some("321"),
                Some("654"),
                "Ativo"
            )
            .unwrap()
            .unwrap()
            .name,
            "Fornecedor Editado"
        );
        assert!(db.set_promotion_active(promotion.id, false).unwrap());
        let promotion = db
            .update_promotion(promotion.id, "Oferta editada", None, "R$ 8,90", true)
            .unwrap()
            .unwrap();
        assert_eq!(promotion.title, "Oferta editada");
        assert_eq!(promotion.price_label, "R$ 8,90");
        assert!(db
            .list_promotions(false)
            .unwrap()
            .iter()
            .any(|x| x.id == promotion.id && x.title == "Oferta editada"));
        let customer = db
            .upsert_customer("Cliente Teste", Some("999"), None, None)
            .unwrap();
        assert!(db
            .list_customers(100)
            .unwrap()
            .iter()
            .any(|x| x.id == customer.id));
        let updated = db
            .update_customer(
                customer.id,
                "Cliente Atualizado",
                Some("999"),
                Some("123"),
                None,
            )
            .unwrap();
        assert_eq!(updated.name, "Cliente Atualizado");
        assert_eq!(updated.phone.as_deref(), Some("123"));
        let order = db.create_purchase_order("78900001", 5.0).unwrap();
        assert!(order > 0);
        drop(db);
        remove_database(&path);
    }

    #[test]
    fn loyalty_rejects_insufficient_balance_and_tracks_points() {
        let (db, path) = temporary_db("loyalty");
        let customer = db
            .upsert_customer("Pontos", Some("998"), None, None)
            .unwrap();
        db.add_points(customer.id, 20, None, Some("crédito"))
            .unwrap();
        assert_eq!(
            db.redeem_points(customer.id, 7, Some("resgate")).unwrap(),
            13
        );
        assert!(db.redeem_points(customer.id, 14, None).is_err());
        drop(db);
        remove_database(&path);
    }

    #[test]
    fn dashboard_and_payment_totals_reflect_committed_sales_only() {
        let (db, path) = temporary_db("dashboard");
        db.cash_open(1000).unwrap();
        let item = SaleItemIn {
            ean: "78900001".into(),
            qty: 1.0,
            price_brl_cents: 3000,
        };
        db.record_sale(
            &[item],
            &PaymentIn {
                method: "pix".into(),
                amount_brl_cents: 3000,
                extra: None,
            },
            "test",
        )
        .unwrap();
        let totals = db.current_payment_totals().unwrap();
        assert_eq!(totals.pix_brl_cents, 3000);
        assert_eq!(totals.cash_brl_cents, 0);
        let summary = db.dashboard_summary().unwrap();
        assert_eq!(summary.sales_today_brl_cents, 3000);
        assert_eq!(summary.items_today, 1);
        let now = chrono::Utc::now().timestamp();
        let rows = db
            .sales_report(
                now - 60,
                now + 60,
                Some("pix"),
                Some("test"),
                Some("78900001"),
            )
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].items_count, 1);
        assert!(db
            .sales_report(now - 60, now + 60, Some("cash"), None, None)
            .unwrap()
            .is_empty());
        drop(db);
        remove_database(&path);
    }

    #[test]
    fn authentication_product_image_and_backup_are_persistent() {
        let (db, path) = temporary_db("auth-backup");
        let admin = db.auth_user("ADMIN").unwrap().expect("admin inicial");
        assert_eq!(admin.role, "admin");
        assert!(admin.must_change_password);
        db.set_user_password(admin.id, "hash-teste", false).unwrap();
        assert_eq!(
            db.auth_user("admin")
                .unwrap()
                .unwrap()
                .password_hash
                .as_deref(),
            Some("hash-teste")
        );
        db.set_product_image("78900001", Some("https://cdn.example/produto.webp"))
            .unwrap();
        assert_eq!(
            db.search_by_ean("78900001")
                .unwrap()
                .unwrap()
                .image_url
                .as_deref(),
            Some("https://cdn.example/produto.webp")
        );
        let backup =
            std::env::temp_dir().join(format!("commercectrl-backup-{}.db", uuid::Uuid::new_v4()));
        db.backup_to(&backup).unwrap();
        Db::validate_backup(&backup).unwrap();
        drop(db);
        remove_database(&path);
        remove_database(&backup);
    }
}
