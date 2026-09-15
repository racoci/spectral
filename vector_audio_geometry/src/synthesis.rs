use crate::{SoundObject, BezierCubic2};
use std::f32::consts::TAU;

pub fn eval_u(curve:&crate::Spline2, t01:f32)->f32 { curve.eval(t01).u }

pub fn synthesize(obj:&SoundObject)->Vec<f32> {
    let n=(obj.duration_s*obj.sample_rate) as usize;
    let mut x=vec![0.0f32;n];
    let mut phases=vec![0.0f32; obj.partials.len()+1];
    let dt=1.0/obj.sample_rate;

    for i in 0..n {
        let t=i as f32*dt;
        let s=(t/obj.duration_s).clamp(0.0,1.0);
        let u0=eval_u(&obj.fundamental,s);
        let f0=20.0*2.0f32.powf(u0);
        let ga=obj.global_amplitude[((obj.global_amplitude.len()-1) as f32*s) as usize];
        phases[0]+=TAU*f0*dt;
        x[i]+=ga*phases[0].cos();
        for (j,p) in obj.partials.iter().enumerate() {
            let idx=((p.relative_log_freq.len()-1) as f32*s) as usize;
            let du=p.relative_log_freq[idx];
            let f=f0*2.0f32.powf(du);
            let a=ga*p.amplitude[idx];
            phases[j+1]+=TAU*f*dt;
            x[i]+=a*(phases[j+1]+p.phase_offset).cos();
        }
    }
    let peak=x.iter().map(|v|v.abs()).fold(0.0,f32::max);
    if peak>0.95 { let q=0.95/peak; for v in &mut x {*v*=q;} }
    x
}
