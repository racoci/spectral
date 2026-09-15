use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use std::time::Instant;
use vector_audio_geometry::*;
use vector_audio_geometry::synthesis::synthesize;
use vector_audio_geometry::reassignment::reassigned_grid;
use vector_audio_geometry::spline_fit::{extract_track_from_grads, fit_track_c2};

fn random_track(rng:&mut impl Rng, frames:usize, fs:f32, hop:usize, lo:f32, hi:f32)->(Vec<f32>,Vec<f32>) {
    let dt=hop as f32/fs;
    let mut u=rng.gen_range(lo.log2()..hi.log2());
    let mut v=0.0f32; let mut a=0.0f32;
    let rho=0.985;
    let mut freq=vec![0.0;frames]; let mut amp=vec![0.0;frames];
    for m in 0..frames {
        if m>0 {
            a=rho*a+(1.0-rho)*rng.gen_range(-1.0..1.0)*8.0;
            v=(v+a*dt)*0.997; u+=v*dt;
            let l=lo.log2(); let h=hi.log2();
            if u<l {u=l;v=v.abs();} if u>h {u=h;v=-v.abs();}
        }
        freq[m]=2.0f32.powf(u);
        amp[m]=(0.8+0.15*rng.gen_range(-1.0..1.0)).max(0.02);
    }
    (freq,amp)
}

fn make_signal(rng:&mut impl Rng, fs:f32, hop:usize, dur:f32)->(Vec<f32>,Vec<f32>,Vec<f32>) {
    let n=(fs*dur) as usize; let frames=(n+hop-1)/hop;
    let (f,a)=random_track(rng,frames,fs,hop,70.0,800.0);
    let dt=1.0/fs; let mut x=vec![0.0;n]; let mut phase=0.0;
    let mut prev=f[0];
    for i in 0..n { let m=(i/hop).min(frames-1); let fm=f[m]; phase+=std::f32::consts::TAU*0.5*(prev+fm)*dt; prev=fm; x[i]+=a[m]*phase.cos(); }
    // Add a soft inharmonic partial following the same fundamental.
    let mut phase2=0.0; prev=f[0];
    for i in 0..n { let m=(i/hop).min(frames-1); let fm=f[m]*2.01; phase2+=std::f32::consts::TAU*fm*dt; x[i]+=0.45*a[m]*phase2.cos(); }
    // Small noise-like texture for adversarial testing.
    for v in &mut x { *v += 0.003*rng.gen_range(-1.0..1.0); }
    (x,f,a)
}

fn rms(v:&[f32])->f64 { (v.iter().map(|x|(*x as f64)*(*x as f64)).sum::<f64>()/(v.len().max(1) as f64)).sqrt() }

fn main(){
    const FS:f32=48_000.0; const HOP:usize=256; const DUR:f32=0.6;
    let seed=rand::random::<u64>(); let mut rng=StdRng::seed_from_u64(seed);
    println!("vector-audio Monte Carlo; seed={seed}");
    let start=Instant::now();
    for case_id in 1u64.. {
        let (x,truth,_amp)=make_signal(&mut rng,FS,HOP,DUR);
        let (channels,grads)=reassigned_grid(&x,FS,HOP,20.0,20_000.0,12,2.0);
        let raw=extract_track_from_grads(&grads,20.0,0.18);
        let spline=fit_track_c2(&raw,24);
        let mut err2=0.0; let mut count=0usize;
        for (m,&f) in truth.iter().enumerate() {
            let t=(m*HOP) as f32/FS; let u=spline.eval((t/(DUR)).clamp(0.0,1.0)).u; let fh=20.0*2.0f32.powf(u);
            let cents=1200.0*((fh/f).max(1e-9)).log2(); err2+=(cents*cents) as f64; count+=1;
        }
        let cents_rms=(err2/count as f64).sqrt();
        if case_id%10==0 { println!("case={case_id:8} channels={} trajectory_rms={cents_rms:8.3} cents elapsed={:.1}s",channels.len(),start.elapsed().as_secs_f32()); }
        if !cents_rms.is_finite() || cents_rms>1000.0 { eprintln!("FAIL case={case_id} rms_cents={cents_rms}"); std::process::exit(1); }
    }
}
