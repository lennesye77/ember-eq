use std::f32::consts::PI;

#[derive(Clone, Copy)]
pub enum BiquadKind { Peak, LowShelf, HighShelf, LowPass, HighPass }

#[derive(Clone, Copy)]
pub struct Coefficients { pub b0:f32, pub b1:f32, pub b2:f32, pub a1:f32, pub a2:f32 }
impl Default for Coefficients { fn default()->Self { Self{b0:1.0,b1:0.0,b2:0.0,a1:0.0,a2:0.0} } }

#[derive(Clone, Copy, Default)]
pub struct Biquad { c:Coefficients, z1:f32, z2:f32 }

impl Biquad {
    pub fn set_coefficients(&mut self,c:Coefficients){self.c=c;}
    pub fn reset(&mut self){self.z1=0.0;self.z2=0.0;}
    #[inline] pub fn process(&mut self,x:f32)->f32 { let y=self.c.b0*x+self.z1; self.z1=self.c.b1*x-self.c.a1*y+self.z2; self.z2=self.c.b2*x-self.c.a2*y; y }
    pub fn coefficients(kind:BiquadKind,sample_rate:f32,frequency:f32,gain_db:f32,q:f32)->Coefficients {
        let f=frequency.clamp(10.0,sample_rate*0.49); let q=q.max(0.05); let w0=2.0*PI*f/sample_rate; let cos=w0.cos(); let sin=w0.sin(); let alpha=sin/(2.0*q); let a=10.0_f32.powf(gain_db/40.0);
        let (b0,b1,b2,a0,a1,a2)=match kind {
            BiquadKind::Peak=>(1.0+alpha*a,-2.0*cos,1.0-alpha*a,1.0+alpha/a,-2.0*cos,1.0-alpha/a),
            BiquadKind::LowPass=>((1.0-cos)*0.5,1.0-cos,(1.0-cos)*0.5,1.0+alpha,-2.0*cos,1.0-alpha),
            BiquadKind::HighPass=>((1.0+cos)*0.5,-(1.0+cos),(1.0+cos)*0.5,1.0+alpha,-2.0*cos,1.0-alpha),
            BiquadKind::LowShelf=>{let s=a.sqrt();let t=2.0*s*alpha;(a*((a+1.0)-(a-1.0)*cos+t),2.0*a*((a-1.0)-(a+1.0)*cos),a*((a+1.0)-(a-1.0)*cos-t),(a+1.0)+(a-1.0)*cos+t,-2.0*((a-1.0)+(a+1.0)*cos),(a+1.0)+(a-1.0)*cos-t)},
            BiquadKind::HighShelf=>{let s=a.sqrt();let t=2.0*s*alpha;(a*((a+1.0)+(a-1.0)*cos+t),-2.0*a*((a-1.0)+(a+1.0)*cos),a*((a+1.0)+(a-1.0)*cos-t),(a+1.0)-(a-1.0)*cos+t,2.0*((a-1.0)-(a+1.0)*cos),(a+1.0)-(a-1.0)*cos-t)}
        };
        Coefficients{b0:b0/a0,b1:b1/a0,b2:b2/a0,a1:a1/a0,a2:a2/a0}
    }
}

#[cfg(test)] mod tests { use super::*; #[test] fn processing_stays_finite(){let mut b=Biquad::default();b.set_coefficients(Biquad::coefficients(BiquadKind::Peak,48000.0,1000.0,6.0,1.0));for i in 0..10000{assert!(b.process(if i==0{1.0}else{0.0}).is_finite());}} }
