use vector_audio_geometry::{Point2,fit_c2};

#[test]
fn c2_spline_is_continuous() {
    let pts=(0..10).map(|i|{let t=i as f32/9.0; Point2{t,u:(3.0*t).sin()}}).collect::<Vec<_>>();
    let s=fit_c2(&pts);
    for i in 0..s.segments.len()-1 {
        let a=s.segments[i].d1(1.0); let b=s.segments[i+1].d1(0.0);
        assert!((a.u-b.u).abs()<1e-4);
    }
}
