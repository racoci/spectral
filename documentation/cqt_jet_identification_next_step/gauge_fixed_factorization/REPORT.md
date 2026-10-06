# Gauge-fixed analytic recovery from Gaussian CQT jet

{
  "true": {
    "harmonic_H_gauge": [
      1.0,
      0.4990768632201854,
      0.2582741459471761,
      0.15090805461199913,
      0.09184109061609423,
      0.0594002998203636
    ],
    "spectral_E_gauge_db": [
      -1.3933588050344992,
      5.744613147148353,
      -0.2549750489115832,
      6.74543675502848,
      0.7458485589685449,
      -1.39129978533418
    ],
    "gauge_affine_slope_db_per_oct": -1.0004118039400638,
    "gauge_offset_db": -2.745024951088417,
    "inharmonicity": 0.000125,
    "pitch_offset_cents": 240,
    "pitch_slope_cents_s": 480,
    "vibrato_depth_cents": 25,
    "vibrato_rate_hz": 5.2,
    "vibrato_phase": 0.37
  },
  "recovery": {
    "harmonic_logH_est": [
      0.0,
      -0.8799987821301477,
      -1.620431373889835,
      -2.27282744192172
    ],
    "harmonic_H_est": [
      1.0,
      0.41478341683349235,
      0.19781334916218501,
      0.10302048346605962
    ],
    "spectral_E_est_db": [
      -4.801235872219051,
      3.372417690068318,
      -0.18056197105755706,
      9.887381484187976,
      7.748557249958521,
      -87.1769423784136
    ],
    "factor_fit_rmse_log": 0.09665677243149266,
    "harmonic_gauge_rmse": 0.05712729759466946,
    "spectral_gauge_rmse_db": 35.20258677876128,
    "factor_matrix_singular_values": [
      12.718052444751844,
      12.554159531063023,
      12.531147259455926,
      1.458686462519363,
      0.851172581503942,
      0.4763338326024564,
      0.07692258920007028,
      0.0035829270576051915,
      1.9389212415480128e-13
    ],
    "factor_rank": 8,
    "inharmonicity_true": 0.000125,
    "inharmonicity_est": 0.00018055573519280902,
    "inharmonicity_relative_error": 0.44444588154247217,
    "pitch_model_est": [
      241.02015237026555,
      479.229727604747,
      10.609881981044417,
      5.194582032892237,
      0.4141428065333385
    ],
    "pitch_model_true": [
      240,
      480,
      25,
      5.2,
      0.37
    ]
  }
}

The affine gauge is fixed by S(440 Hz)=0 and dS/d(log2 f)|440=0. Without this gauge, the factorization has an unavoidable null direction corresponding to transferring a power-law between harmonic structure, spectral envelope, and common note amplitude.
