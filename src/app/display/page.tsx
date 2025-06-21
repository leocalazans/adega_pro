'use client';

import { useState, useEffect, useRef } from 'react';
import Image from 'next/image';
import { getPromotions } from '@/lib/data';
import type { Promotion } from '@/lib/types';
import { Skeleton } from '@/components/ui/skeleton';
import { Button } from '@/components/ui/button';
import { Volume2, VolumeX, MonitorPlay, Image as ImageIcon } from 'lucide-react';
import { cn } from '@/lib/utils';

const PROMOTION_INTERVAL = 8000; // 8 seconds
const MUSIC_URL = "https://cdn.pixabay.com/download/audio/2022/08/04/audio_2d891b22e1.mp3"; // Royalty-free Lofi
const VIDEO_URL = "https://videos.pexels.com/video-files/2759477/2759477-hd_1920_1080_25fps.mp4"; // Royalty-free video of people at a bar

export default function DisplayPage() {
  const [promotions, setPromotions] = useState<Promotion[]>([]);
  const [currentPromotionIndex, setCurrentPromotionIndex] = useState(0);
  const [isLoading, setIsLoading] = useState(true);
  const [viewMode, setViewMode] = useState<'promotions' | 'video'>('promotions');
  const [isMuted, setIsMuted] = useState(true);
  const [isInteracted, setIsInteracted] = useState(false);
  const audioRef = useRef<HTMLAudioElement>(null);

  useEffect(() => {
    const fetchPromos = async () => {
      try {
        const promos = await getPromotions();
        setPromotions(promos);
      } catch (e) {
        console.error("Failed to load promotions", e);
      } finally {
        setIsLoading(false);
      }
    };
    fetchPromos();
  }, []);

  useEffect(() => {
    if (promotions.length > 1) {
      const timer = setInterval(() => {
        setCurrentPromotionIndex(prevIndex => (prevIndex + 1) % promotions.length);
      }, PROMOTION_INTERVAL);
      return () => clearInterval(timer);
    }
  }, [promotions]);
  
  useEffect(() => {
      if (audioRef.current) {
          audioRef.current.muted = isMuted;
          if(!isMuted && isInteracted) {
            audioRef.current.play().catch(e => console.error("Audio play failed:", e));
          }
      }
  }, [isMuted, isInteracted]);
  
  const handleUserInteraction = () => {
      if (!isInteracted) {
          setIsInteracted(true);
          if (audioRef.current) {
            audioRef.current.play().catch(e => console.error("Audio play failed on interaction:", e));
          }
      }
  }
  
  const toggleMute = () => {
      handleUserInteraction();
      setIsMuted(prev => !prev);
  }

  const toggleViewMode = () => {
      handleUserInteraction();
      setViewMode(prev => prev === 'promotions' ? 'video' : 'promotions');
  }
  
  if (isLoading) {
    return (
      <div className="h-screen w-screen bg-black flex items-center justify-center">
        <Skeleton className="w-4/5 h-4/5" />
      </div>
    );
  }

  const currentPromotion = promotions[currentPromotionIndex];

  return (
    <div className="relative h-screen w-screen overflow-hidden bg-black text-white font-sans" onClick={handleUserInteraction}>
      <style jsx global>{`
        .font-anton { font-family: 'Anton', sans-serif; }
        .font-poppins { font-family: 'Poppins', sans-serif; }
        .text-stroke {
          -webkit-text-stroke: 2px black;
          paint-order: stroke fill;
        }
        @keyframes fadeIn {
          from { opacity: 0; }
          to { opacity: 1; }
        }
        @keyframes slideInUp {
          from { transform: translateY(50px); opacity: 0; }
          to { transform: translateY(0); opacity: 1; }
        }
        .animate-fade-in { animation: fadeIn 1s ease-in-out; }
        .animate-slide-in-up { animation: slideInUp 1s ease-out; }
      `}</style>

      {viewMode === 'video' && (
        <video
          key="bg-video"
          className="absolute top-0 left-0 w-full h-full object-cover animate-fade-in"
          autoPlay
          loop
          muted
        >
          <source src={VIDEO_URL} type="video/mp4" />
        </video>
      )}

      {viewMode === 'promotions' && currentPromotion && (
         <div key={currentPromotion.id} className="w-full h-full animate-fade-in">
          <Image
            src={currentPromotion.image}
            alt={currentPromotion.productName}
            layout="fill"
            objectFit="cover"
            className="z-0 opacity-70"
            data-ai-hint={currentPromotion.hint}
          />
          <div className="absolute inset-0 bg-gradient-to-t from-black/80 via-black/20 to-black/50 z-10" />
          <div className="absolute inset-0 z-20 flex flex-col justify-end items-center text-center p-8 md:p-16">
             <div className="animate-slide-in-up" style={{ animationDelay: '0.2s' }}>
                {currentPromotion.highlight && (
                    <span className="inline-block bg-primary text-primary-foreground font-poppins font-bold text-2xl md:text-4xl px-6 py-2 rounded-full mb-4 tracking-wider">
                        {currentPromotion.highlight}
                    </span>
                )}
                <h1 className="font-anton text-6xl md:text-9xl lg:text-[10rem] leading-none text-stroke text-white uppercase" style={{ textShadow: '4px 4px 10px rgba(0,0,0,0.5)' }}>
                    {currentPromotion.productName}
                </h1>
                <p className="font-poppins font-black text-5xl md:text-8xl lg:text-[9rem] text-yellow-300 leading-tight text-stroke" style={{ textShadow: '4px 4px 10px rgba(0,0,0,0.5)' }}>
                    {currentPromotion.price}
                </p>
             </div>
          </div>
        </div>
      )}

      <audio ref={audioRef} loop>
        <source src={MUSIC_URL} type="audio/mpeg" />
      </audio>

      <div className="absolute bottom-4 right-4 z-30 flex gap-3">
         <Button onClick={toggleViewMode} variant="secondary" size="icon" className="rounded-full h-12 w-12 bg-black/50 hover:bg-black/80 border-white/20 border">
            {viewMode === 'promotions' ? <MonitorPlay /> : <ImageIcon />}
         </Button>
         <Button onClick={toggleMute} variant="secondary" size="icon" className="rounded-full h-12 w-12 bg-black/50 hover:bg-black/80 border-white/20 border">
            {isMuted ? <VolumeX /> : <Volume2 />}
         </Button>
      </div>
    </div>
  );
}
