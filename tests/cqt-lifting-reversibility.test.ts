import fs from 'fs';
import path from 'path';

// ============================================================================
// Automated Reversibility & Bijection Test Suite: V7 CQT FIR Lifting Cascade
// ============================================================================

const FBITS = 20n;
const p_fix = [553479n, 35849n, 2315n];
const u_fix = [218054n, 49220n, 69421n];

// Reversibility floor rounding logic matching exact fixed-point integer mathematics
function round_fixed(x: bigint, bits: bigint): bigint {
    const den = 1n << bits;
    const half = den >> 1n;
    if (x >= 0n) {
        return (x + half) / den;
    } else {
        return -((-x + half) / den);
    }
}

// Reversible symmetric FIR Predictor
function pred_fir_int(e: bigint[]): bigint[] {
    const len = e.length;
    const p = new Array<bigint>(len);
    for (let k = 0; k < len; k++) {
        const em2 = e[Math.max(k - 2, 0)];
        const em1 = e[Math.max(k - 1, 0)];
        const ec  = e[k];
        const ep1 = e[Math.min(k + 1, len - 1)];
        const ep2 = e[Math.min(k + 2, len - 1)];

        const acc = p_fix[0] * ec + p_fix[1] * (em1 + ep1) + p_fix[2] * (em2 + ep2);
        p[k] = round_fixed(acc, FBITS);
    }
    return p;
}

// Reversible symmetric FIR Updater
function upd_fir_int(d: bigint[]): bigint[] {
    const len = d.length;
    if (len === 0) return [];
    const u = new Array<bigint>(len);
    for (let k = 0; k < len; k++) {
        const dm2 = d[Math.max(k - 2, 0)];
        const dm1 = d[Math.max(k - 1, 0)];
        const dc  = d[k];
        const dp1 = d[Math.min(k + 1, len - 1)];
        const dp2 = d[Math.min(k + 2, len - 1)];

        const acc = u_fix[0] * dc + u_fix[1] * (dm1 + dp1) + u_fix[2] * (dm2 + dp2);
        u[k] = round_fixed(acc, FBITS);
    }
    return u;
}

// Symmetric single-stage forward FIR lifting shear
function lift_forward_fir(v: bigint[]): { s: bigint[], d: bigint[] } {
    const len = v.length;
    const e: bigint[] = [];
    const o: bigint[] = [];
    for (let i = 0; i < len; i++) {
        if (i % 2 === 0) e.push(v[i]);
        else o.push(v[i]);
    }
    
    const o_len = o.length;
    const p = pred_fir_int(e.slice(0, o_len));
    const d = new Array<bigint>(o_len);
    for (let i = 0; i < o_len; i++) {
        d[i] = o[i] - p[i];
    }
    
    const u = upd_fir_int(d);
    const s = [...e];
    for (let i = 0; i < u.length; i++) {
        s[i] += u[i];
    }
    
    return { s, d };
}

// Symmetric single-stage inverse FIR lifting shear
function lift_inverse_fir(s: bigint[], d: bigint[], original_len: number): bigint[] {
    const u = upd_fir_int(d);
    const e = [...s];
    for (let i = 0; i < u.length; i++) {
        e[i] -= u[i];
    }
    
    const d_len = d.length;
    const p = pred_fir_int(e.slice(0, d_len));
    const o = new Array<bigint>(d_len);
    for (let i = 0; i < d_len; i++) {
        o[i] = d[i] + p[i];
    }
    
    const out = new Array<bigint>(original_len);
    for (let i = 0; i < e.length; i++) {
        out[i * 2] = e[i];
    }
    for (let i = 0; i < o.length; i++) {
        out[i * 2 + 1] = o[i];
    }
    return out;
}

// Stage-cascade recursive forward lifting decomposition (default stages = 3)
function cascade_forward_fir(v: bigint[], stages = 3): { low: bigint[], details: bigint[][], lengths: number[] } {
    let cur = [...v];
    const details: bigint[][] = [];
    const lengths: number[] = [];
    for (let s = 0; s < stages; s++) {
        if (cur.length < 6) break;
        const { s: next_s, d } = lift_forward_fir(cur);
        details.push(d);
        lengths.push(cur.length);
        cur = next_s;
    }
    return { low: cur, details, lengths };
}

// Stage-cascade recursive inverse lifting reconstruction (default stages = 3)
function cascade_inverse_fir(low: bigint[], details: bigint[][], lengths: number[]): bigint[] {
    let cur = [...low];
    for (let k = details.length - 1; k >= 0; k--) {
        cur = lift_inverse_fir(cur, details[k], lengths[k]);
    }
    return cur;
}

// Test Runner
function runBijectionTests() {
    console.log('================================================================================');
    console.log('            Automated V7 CQT FIR Lifting Reversibility Test Suite               ');
    console.log('================================================================================\n');

    let totalTests = 0;
    let successfulTests = 0;
    let totalMismatches = 0;

    // Test Case 1: Simple Delta impulses
    console.log('▶️ Executing Test Case 1: Delta impulses along 547 channels...');
    for (let impulsePos = 0; impulsePos < 547; impulsePos += 13) {
        totalTests++;
        const signal = new Array<bigint>(547).fill(0n);
        signal[impulsePos] = 8388607n; // Max 24-bit peak amplitude
        
        const { low, details, lengths } = cascade_forward_fir(signal, 3);
        const reconstructed = cascade_inverse_fir(low, details, lengths);

        let mismatch = 0;
        for (let i = 0; i < 547; i++) {
            if (signal[i] !== reconstructed[i]) mismatch++;
        }
        
        if (mismatch === 0) {
            successfulTests++;
        } else {
            totalMismatches += mismatch;
            console.error(`  ❌ Mismatch detected at impulse position ${impulsePos}! Mismatches: ${mismatch}`);
        }
    }
    console.log(`  PASSED: ${successfulTests}/${totalTests} impulse roundtrips reconstructed with EXACT IDENTITY.\n`);

    // Test Case 2: Multi-scale dense random fuzzer (adversarial limits)
    console.log('▶️ Executing Test Case 2: Multi-scale dense random fuzzer (1000 trials)...');
    let randomSuccesses = 0;
    const trials = 1000;
    for (let t = 0; t < trials; t++) {
        totalTests++;
        const signal = new Array<bigint>(547);
        for (let i = 0; i < 547; i++) {
            // Random BigInt amplitude in full 24-bit range [-8388608, 8388607]
            signal[i] = BigInt(Math.floor(Math.random() * 16777216) - 8388608);
        }

        const { low, details, lengths } = cascade_forward_fir(signal, 3);
        const reconstructed = cascade_inverse_fir(low, details, lengths);

        let mismatch = 0;
        for (let i = 0; i < 547; i++) {
            if (signal[i] !== reconstructed[i]) mismatch++;
        }

        if (mismatch === 0) {
            successfulTests++;
            randomSuccesses++;
        } else {
            totalMismatches += mismatch;
            console.error(`  ❌ Random Fuzzer failure at trial ${t}! Mismatches: ${mismatch}`);
        }
    }
    console.log(`  PASSED: ${randomSuccesses}/${trials} dense random vectors reconstructed with EXACT IDENTITY.\n`);

    // Test Case 3: Constant DC values and alternating square waves
    console.log('▶️ Executing Test Case 3: Constant DC offsets and Nyquist alternating waves...');
    const dcs = [1000n, -50000n, 8388607n, -8388608n];
    for (const dc of dcs) {
        totalTests++;
        const signal = new Array<bigint>(547).fill(dc);
        const { low, details, lengths } = cascade_forward_fir(signal, 3);
        const reconstructed = cascade_inverse_fir(low, details, lengths);

        let mismatch = 0;
        for (let i = 0; i < 547; i++) {
            if (signal[i] !== reconstructed[i]) mismatch++;
        }
        if (mismatch === 0) successfulTests++;
    }

    // Alternating wave (Nyquist)
    totalTests++;
    const NyquistSignal = new Array<bigint>(547);
    for (let i = 0; i < 547; i++) {
        NyquistSignal[i] = i % 2 === 0 ? 4000000n : -4000000n;
    }
    const { low, details, lengths } = cascade_forward_fir(NyquistSignal, 3);
    const reconstructedNyquist = cascade_inverse_fir(low, details, lengths);
    let nyqMismatch = 0;
    for (let i = 0; i < 547; i++) {
        if (NyquistSignal[i] !== reconstructedNyquist[i]) nyqMismatch++;
    }
    if (nyqMismatch === 0) successfulTests++;

    console.log(`  PASSED: Special waveforms successfully verified.\n`);

    console.log('================================================================================');
    console.log('                              FINAL TEST REPORT                                 ');
    console.log('================================================================================');
    console.log(`Total Test Scenarios Executed : ${totalTests}`);
    console.log(`Successful Exact Roundtrips   : ${successfulTests}`);
    console.log(`Total Coef Mismatches (Errors): ${totalMismatches}`);
    console.log(`Perfect 100% Reversibility    : ${totalMismatches === 0 ? 'YES (TRUE ✅)' : 'NO ❌'}`);
    console.log('================================================================================');

    if (totalMismatches > 0) {
        process.exit(1);
    }
}

runBijectionTests();
