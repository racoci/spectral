use crate::phasegrad::{Grad, Channel, log_channels, analyze};

pub fn reassigned_grid(input:&[f32], fs:f32, hop:usize, fmin:f32, fmax:f32, bins_per_octave:usize, cycles:f32)
-> (Vec<Channel>, Vec<Vec<Grad>>) {
    let ch=log_channels(fmin,fmax,bins_per_octave,cycles);
    let g=analyze(input,fs,hop,&ch);
    (ch,g)
}
