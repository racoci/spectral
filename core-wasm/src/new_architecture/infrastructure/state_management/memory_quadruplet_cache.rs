//! Cache Global de Quadrupletos em Memória (LOD Progressivo)
//!
//! Armazenamento estático em memória linear dos quadrupletos de dados espectrais (re, im, d_phi_dt, d_phi_dw)
//! para permitir renderização progressiva instantânea sem recalcular FFTs.

use std::sync::RwLock;

static GLOBAL_QUADRUPLET_CACHE: RwLock<Option<CachedQuadrupletsData>> = RwLock::new(None);

pub struct CachedQuadrupletsData {
    pub width: usize,
    pub height: usize,
    pub data: Vec<f32>,
}

pub struct MemoryQuadrupletCacheManager;

impl MemoryQuadrupletCacheManager {
    pub fn store(width: usize, height: usize, data: Vec<f32>) {
        if let Ok(mut lock) = GLOBAL_QUADRUPLET_CACHE.write() {
            *lock = Some(CachedQuadrupletsData { width, height, data });
        }
    }

    pub fn has_cached_data() -> bool {
        if let Ok(lock) = GLOBAL_QUADRUPLET_CACHE.read() {
            lock.is_some()
        } else {
            false
        }
    }

    pub fn clear() {
        if let Ok(mut lock) = GLOBAL_QUADRUPLET_CACHE.write() {
            *lock = None;
        }
    }
}
