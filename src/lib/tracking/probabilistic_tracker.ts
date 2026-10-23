export interface RidgeState4D {
  f: number;
  fDot: number;
  lnA: number;
  aDotRel: number;
}

export interface ObservedPeak {
  f: number;
  lnA: number;
}

export interface FilterConfig {
  f: number;
  fDot?: number;
  lnA?: number;
  aDotRel?: number;
  dt?: number;
  sigmaF?: number;
  sigmaLnA?: number;
}

/**
 * Filtro de Kalman 4D para dinâmica de cristas espectrais no browser.
 */
export class KalmanRidgeFilter {
  public state: RidgeState4D;
  public dt: number;
  public P: number[][]; // Covariância 4x4
  private Q: number[][]; // Ruído de processo
  private R: number[][]; // Ruído de observação

  constructor(cfg: FilterConfig) {
    this.dt = cfg.dt || 0.005;
    this.state = {
      f: cfg.f,
      fDot: cfg.fDot || 0.0,
      lnA: cfg.lnA !== undefined ? cfg.lnA : 0.0,
      aDotRel: cfg.aDotRel || 0.0
    };

    const sF = cfg.sigmaF || 2.0;
    const sA = cfg.sigmaLnA || 0.05;

    this.P = [
      [sF * sF, 0, 0, 0],
      [0, (sF / this.dt) * (sF / this.dt), 0, 0],
      [0, 0, sA * sA, 0],
      [0, 0, 0, 1.0]
    ];

    const qF = 60.0 * 60.0;
    const qA = 1.5 * 1.5;
    const dt2 = this.dt * this.dt;
    const dt3 = this.dt * dt2;

    this.Q = [
      [(dt3 / 3.0) * qF, (dt2 / 2.0) * qF, 0, 0],
      [(dt2 / 2.0) * qF, this.dt * qF, 0, 0],
      [0, 0, (dt3 / 3.0) * qA, (dt2 / 2.0) * qA],
      [0, 0, (dt2 / 2.0) * qA, this.dt * qA]
    ];

    this.R = [
      [sF * sF, 0],
      [0, sA * sA]
    ];
  }

  public predict() {
    this.state.f += this.state.fDot * this.dt;
    this.state.lnA += this.state.aDotRel * this.dt;

    // Atualiza P = F P F^T + Q
    const p00 = this.P[0][0] + 2 * this.dt * this.P[0][1] + this.dt * this.dt * this.P[1][1] + this.Q[0][0];
    const p01 = this.P[0][1] + this.dt * this.P[1][1] + this.Q[0][1];
    const p11 = this.P[1][1] + this.Q[1][1];

    const p22 = this.P[2][2] + 2 * this.dt * this.P[2][3] + this.dt * this.dt * this.P[3][3] + this.Q[2][2];
    const p23 = this.P[2][3] + this.dt * this.P[3][3] + this.Q[2][3];
    const p33 = this.P[3][3] + this.Q[3][3];

    this.P[0][0] = p00;
    this.P[0][1] = p01;
    this.P[1][0] = p01;
    this.P[1][1] = p11;

    this.P[2][2] = p22;
    this.P[2][3] = p23;
    this.P[3][2] = p23;
    this.P[3][3] = p33;
  }

  public update(obs: ObservedPeak) {
    const yF = obs.f - this.state.f;
    const yA = obs.lnA - this.state.lnA;

    const sF = this.P[0][0] + this.R[0][0] + 1e-6;
    const sA = this.P[2][2] + this.R[1][1] + 1e-6;

    const kF0 = this.P[0][0] / sF;
    const kF1 = this.P[1][0] / sF;

    const kA2 = this.P[2][2] / sA;
    const kA3 = this.P[3][2] / sA;

    this.state.f += kF0 * yF;
    this.state.fDot += kF1 * yF;
    this.state.lnA += kA2 * yA;
    this.state.aDotRel += kA3 * yA;

    this.P[0][0] *= (1.0 - kF0);
    this.P[1][1] -= kF1 * this.P[0][1];
    this.P[2][2] *= (1.0 - kA2);
    this.P[3][3] -= kA3 * this.P[2][3];
  }

  public getInnovationCovariance(): { sF: number; sA: number } {
    return {
      sF: this.P[0][0] + this.R[0][0] + 1e-6,
      sA: this.P[2][2] + this.R[1][1] + 1e-6
    };
  }
}

/**
 * Associador baseado na distância quadrada de Mahalanobis.
 */
export class MahalanobisAssociator {
  constructor(public gateThreshold = 9.21) {}

  public computeMahalanobisDistance(filter: KalmanRidgeFilter, peak: ObservedPeak): number {
    const diffF = peak.f - filter.state.f;
    const diffA = peak.lnA - filter.state.lnA;
    const { sF, sA } = filter.getInnovationCovariance();
    return (diffF * diffF) / sF + (diffA * diffA) / sA;
  }
}
