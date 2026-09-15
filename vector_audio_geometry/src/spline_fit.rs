use crate::geometry::{Point2,BezierCubic2,Spline2,hermite_to_bezier};

/// Natural cubic spline interpolation of samples y(x), returned as cubic Bezier segments.
/// The x coordinates must be strictly increasing.
pub fn fit_c2(points:&[Point2])->Spline2 {
    assert!(points.len()>=2);
    for w in points.windows(2) { assert!(w[1].t>w[0].t); }
    let n=points.len();
    if n==2 {
        let d=(points[1]-points[0])/(points[1].t-points[0].t);
        return Spline2{segments:vec![hermite_to_bezier(points[0],points[1],d,d,points[1].t-points[0].t)]};
    }

    let mut h=vec![0.0f32;n-1];
    for i in 0..n-1 { h[i]=points[i+1].t-points[i].t; }

    // Natural cubic spline second derivatives m[i].
    let mut a=vec![0.0f32;n];
    let mut b=vec![0.0f32;n];
    let mut c=vec![0.0f32;n];
    let mut r=vec![0.0f32;n];
    b[0]=1.0; b[n-1]=1.0;
    for i in 1..n-1 {
        a[i]=h[i-1];
        b[i]=2.0*(h[i-1]+h[i]);
        c[i]=h[i];
        r[i]=6.0*((points[i+1].u-points[i].u)/h[i]-(points[i].u-points[i-1].u)/h[i-1]);
    }
    // Thomas algorithm.
    for i in 1..n {
        let q=a[i]/b[i-1];
        b[i]-=q*c[i-1];
        r[i]-=q*r[i-1];
    }
    let mut m=vec![0.0f32;n];
    m[n-1]=r[n-1]/b[n-1];
    for i in (0..n-1).rev() { m[i]=(r[i]-c[i]*m[i+1])/b[i]; }

    let mut segs=Vec::with_capacity(n-1);
    for i in 0..n-1 {
        let hi=h[i];
        let d0=(points[i+1].u-points[i].u)/hi-hi*(2.0*m[i]+m[i+1])/6.0;
        let d1=(points[i+1].u-points[i].u)/hi+hi*(m[i]+2.0*m[i+1])/6.0;
        let p0=points[i];
        let p3=points[i+1];
        let bcurve=BezierCubic2 {
            p0,
            p1: Point2{t:p0.t+hi/3.0,u:p0.u+d0*hi/3.0},
            p2: Point2{t:p3.t-hi/3.0,u:p3.u-d1*hi/3.0},
            p3,
        };
        segs.push(bcurve);
    }
    Spline2{segments:segs}
}

pub fn sample_uniform(grid_t:&[f32], grid_u:&[f32], target_points:usize)->Vec<Point2> {
    assert_eq!(grid_t.len(),grid_u.len());
    assert!(target_points>=2);
    if target_points==grid_t.len() { return grid_t.iter().zip(grid_u).map(|(&t,&u)|Point2{t,u}).collect(); }
    let mut out=Vec::with_capacity(target_points);
    for j in 0..target_points {
        let q=j as f32*(grid_t.len()-1) as f32/(target_points-1) as f32;
        let i=q.floor() as usize;
        let a=q-i as f32;
        if i+1>=grid_t.len() { out.push(Point2{t:grid_t[i],u:grid_u[i]}); }
        else { out.push(Point2{t:grid_t[i]*(1.0-a)+grid_t[i+1]*a,u:grid_u[i]*(1.0-a)+grid_u[i+1]*a}); }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn interpolation_and_c2_endpoints() {
        let p:Vec<_>=(0..8).map(|i|{let t=i as f32/7.0; Point2{t,u:(2.0*t).sin()*0.2}}).collect();
        let s=fit_c2(&p);
        for (seg,q) in s.segments.iter().zip(p.windows(2)) {
            assert!((seg.eval(0.0).t-q[0].t).abs()<1e-6);
            assert!((seg.eval(1.0).t-q[1].t).abs()<1e-6);
        }
        for i in 0..s.segments.len()-1 {
            let a=s.segments[i].d1(1.0); let b=s.segments[i+1].d1(0.0);
            assert!((a.t-b.t).abs()<1e-5);
            assert!((a.u-b.u).abs()<1e-5);
        }
    }
}

/// Extract a preliminary track from reassigned coefficients.
/// For each frame, select an energy-weighted centroid in log-frequency around the strongest channel.
pub fn extract_track_from_grads(
    grads:&[Vec<crate::phasegrad::Grad>],
    fmin:f32,
    max_oct_radius:f32,
) -> Vec<Point2> {
    assert!(!grads.is_empty());
    let channels=grads.len();
    let frames=grads[0].len();
    let mut out=Vec::with_capacity(frames);
    for m in 0..frames {
        let mut best_k=0usize;
        let mut best_e=-1.0f32;
        for k in 0..channels {
            let g=grads[k][m];
            let e=g.coeff.abs2();
            if e>best_e { best_e=e; best_k=k; }
        }
        let center_u=(grads[best_k][m].freq_hz/fmin).max(1e-9).log2();
        let mut sw=0.0f32; let mut su=0.0f32;
        for k in 0..channels {
            let g=grads[k][m];
            if !g.freq_hz.is_finite() || g.freq_hz<=0.0 { continue; }
            let u=(g.freq_hz/fmin).log2();
            if (u-center_u).abs()<=max_oct_radius {
                let w=g.coeff.abs2();
                sw+=w; su+=w*u;
            }
        }
        let u=if sw>0.0 { su/sw } else { center_u };
        let t=grads[0][m].time_s;
        out.push(Point2{t,u});
    }
    out
}

/// Fit a C2 spline to a track after uniform decimation. This intentionally keeps
/// the vector representation small; the dense samples remain available for error metrics.
pub fn fit_track_c2(raw:&[Point2], control_points:usize)->Spline2 {
    let p=sample_uniform(
        &raw.iter().map(|x|x.t).collect::<Vec<_>>(),
        &raw.iter().map(|x|x.u).collect::<Vec<_>>(),
        control_points.max(2),
    );
    fit_c2(&p)
}
