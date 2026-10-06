use std::f32::consts::PI;

/// Árvore de Sintaxe Abstrata (AST) para avaliar qualquer parâmetro de síntese no tempo.
///
/// Isso forma um "Grafo de Modulação", permitindo que frequência, amplitude e
/// inarmonicidade sejam controladas por cadeias de LFOs, Envelopes e Curvas,
/// com profundidade e largura arbitrárias.
#[derive(Clone, Debug)]
pub enum ModExpr {
    /// Constante literal
    Const(f32),
    /// Tempo `t` atual
    Time,
    /// Soma de duas modulações
    Add(Box<ModExpr>, Box<ModExpr>),
    /// Multiplicação de duas modulações
    Mul(Box<ModExpr>, Box<ModExpr>),
    /// Seno contínuo
    Sin(Box<ModExpr>),
    /// Envelope Clássico ADSR Exponencial/Linear
    Adsr {
        attack_time: f32,
        decay_time: f32,
        sustain_level: f32,
        release_time: f32,
        t_on: f32,
        t_off: f32,
    },
    /// Oscilador LFO
    Lfo {
        freq: Box<ModExpr>,
        depth: Box<ModExpr>,
        phase_offset: f32,
    },
}

impl ModExpr {
    /// Avalia a árvore de expressão no instante `t`.
    pub fn eval(&self, t: f32) -> f32 {
        match self {
            ModExpr::Const(c) => *c,
            ModExpr::Time => t,
            ModExpr::Add(a, b) => a.eval(t) + b.eval(t),
            ModExpr::Mul(a, b) => a.eval(t) * b.eval(t),
            ModExpr::Sin(a) => a.eval(t).sin(),
            ModExpr::Lfo { freq, depth, phase_offset } => {
                let f = freq.eval(t);
                let d = depth.eval(t);
                d * (2.0 * PI * f * t + phase_offset).sin()
            }
            ModExpr::Adsr { attack_time, decay_time, sustain_level, release_time, t_on, t_off } => {
                if t < *t_on {
                    0.0
                } else if t < *t_off {
                    // Fase ON (Attack, Decay, Sustain)
                    let rel_t = t - t_on;
                    if rel_t < *attack_time {
                        // Ataque linear
                        rel_t / attack_time
                    } else if rel_t < attack_time + decay_time {
                        // Decaimento
                        let decay_progress = (rel_t - attack_time) / decay_time;
                        1.0 - decay_progress * (1.0 - sustain_level)
                    } else {
                        // Sustain
                        *sustain_level
                    }
                } else {
                    // Fase OFF (Release)
                    let rel_t = t - t_off;
                    if rel_t < *release_time {
                        let release_progress = rel_t / release_time;
                        // Nível em que a nota foi solta (assume-se sustain se chegou até aqui)
                        sustain_level * (1.0 - release_progress)
                    } else {
                        0.0
                    }
                }
            }
        }
    }

    /// Atalhos convenientes para construção do AST
    pub fn add(self, other: ModExpr) -> ModExpr { ModExpr::Add(Box::new(self), Box::new(other)) }
    pub fn mul(self, other: ModExpr) -> ModExpr { ModExpr::Mul(Box::new(self), Box::new(other)) }
    pub fn constant(v: f32) -> ModExpr { ModExpr::Const(v) }
}

/// Um evento de nota que será executado pelo sintetizador
#[derive(Clone, Debug)]
pub struct NoteEvent {
    pub t_on: f32,
    pub t_off: f32,
    pub f0_base: f32,
    pub velocity: f32,
}

/// Configuração do timbre harmônico de uma voz no sintetizador.
#[derive(Clone, Debug)]
pub struct VoiceConfig {
    /// Árvore de modulação global para a frequência base (ex: vibrato, portamento)
    pub f0_mod: ModExpr,
    /// Árvore de modulação global para a amplitude (ex: tremolo, ADSR master)
    pub amp_mod: ModExpr,
    /// Inarmonicidade (B) - 0.0 é harmônico perfeito
    pub inharmonicity_b: ModExpr,
    /// Quantidade de parciais
    pub num_harmonics: usize,
    /// Modulações de amplitude para cada harmônico individual `H_k(t)`
    pub harmonic_amps: Vec<ModExpr>,
}

/// O Sintetizador Ground Truth
pub struct GroundTruthSynth {
    pub sample_rate: f32,
    pub voice: VoiceConfig,
    pub events: Vec<NoteEvent>,
}

impl GroundTruthSynth {
    /// Renderiza o sinal PCM combinando todas as modulações da árvore AST e integrando
    /// a fase analiticamente em precisão de amostra.
    pub fn render(&self, duration_s: f32) -> Vec<f32> {
        let n_samples = (duration_s * self.sample_rate).ceil() as usize;
        let mut buffer = vec![0.0f32; n_samples];
        let dt = 1.0 / self.sample_rate;

        // Para simplificar, assumimos síntese monofônica contínua para um único VoiceConfig
        // mas a fase deve ser acumulada continuamente (integração de f_inst)
        
        let mut phases = vec![0.0f32; self.voice.num_harmonics];

        for i in 0..n_samples {
            let t = i as f32 * dt;

            // Encontrar qual nota está ativa ou liberar (só consideramos 1 voz tocando por vez neste exemplo)
            let mut active_note: Option<&NoteEvent> = None;
            for ev in &self.events {
                // Consideramos que o release pode estender o som
                if t >= ev.t_on && t < ev.t_off + 2.0 { 
                    active_note = Some(ev);
                    break;
                }
            }

            if let Some(note) = active_note {
                let f0_val = note.f0_base + self.voice.f0_mod.eval(t);
                let amp_val = note.velocity * self.voice.amp_mod.eval(t);
                let b_val = self.voice.inharmonicity_b.eval(t);

                let mut sample_out = 0.0f32;

                for k in 1..=self.voice.num_harmonics {
                    let k_f32 = k as f32;
                    let h_amp = self.voice.harmonic_amps[k - 1].eval(t);
                    
                    // Cálculo da inarmonicidade: f_k(t) = k * f_0 * sqrt(1 + B * k^2)
                    let inharmonic_factor = (1.0 + b_val * k_f32 * k_f32).sqrt();
                    let f_k = k_f32 * f0_val * inharmonic_factor;

                    // Integração contínua da fase da portadora
                    phases[k - 1] += 2.0 * PI * f_k * dt;
                    
                    sample_out += h_amp * phases[k - 1].sin();
                }

                buffer[i] += amp_val * sample_out;
            }
        }

        // Normalização / Clipping macio
        let mut peak = 0.0f32;
        for &s in &buffer {
            if s.abs() > peak {
                peak = s.abs();
            }
        }
        if peak > 0.95 {
            let scaler = 0.95 / peak;
            for s in &mut buffer {
                *s *= scaler;
            }
        }

        buffer
    }
}
