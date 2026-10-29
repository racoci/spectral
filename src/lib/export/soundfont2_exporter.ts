/**
 * Exportador de SoundFont 2.04 (.sf2) binário autocontido compatível com DAWs e Samplers.
 */

export interface Sf2SampleDescriptor {
  name: string;
  rootKey: number;
  keyLow: number;
  keyHigh: number;
  pcm: Int16Array;
  sampleRate: number;
}

class ByteWriter {
  private buffer: Uint8Array;
  private offset = 0;

  constructor(initialCapacity = 65536) {
    this.buffer = new Uint8Array(initialCapacity);
  }

  private ensureCapacity(extra: number) {
    if (this.offset + extra > this.buffer.length) {
      const newCap = Math.max(this.buffer.length * 2, this.offset + extra + 65536);
      const newBuf = new Uint8Array(newCap);
      newBuf.set(this.buffer);
      this.buffer = newBuf;
    }
  }

  public writeString(str: string, fixedLength?: number): void {
    if (fixedLength !== undefined) {
      this.ensureCapacity(fixedLength);
      for (let i = 0; i < fixedLength; i++) {
        const code = i < str.length ? str.charCodeAt(i) : 0;
        this.buffer[this.offset++] = code & 0xFF;
      }
    } else {
      this.ensureCapacity(str.length);
      for (let i = 0; i < str.length; i++) {
        this.buffer[this.offset++] = str.charCodeAt(i) & 0xFF;
      }
    }
  }

  public writeUint8(val: number): void {
    this.ensureCapacity(1);
    this.buffer[this.offset++] = val & 0xFF;
  }

  public writeUint16(val: number): void {
    this.ensureCapacity(2);
    this.buffer[this.offset++] = val & 0xFF;
    this.buffer[this.offset++] = (val >> 8) & 0xFF;
  }

  public writeInt16(val: number): void {
    this.ensureCapacity(2);
    const u = val < 0 ? val + 65536 : val;
    this.buffer[this.offset++] = u & 0xFF;
    this.buffer[this.offset++] = (u >> 8) & 0xFF;
  }

  public writeUint32(val: number): void {
    this.ensureCapacity(4);
    this.buffer[this.offset++] = val & 0xFF;
    this.buffer[this.offset++] = (val >> 8) & 0xFF;
    this.buffer[this.offset++] = (val >> 16) & 0xFF;
    this.buffer[this.offset++] = (val >> 24) & 0xFF;
  }

  public writeBytes(bytes: Uint8Array): void {
    this.ensureCapacity(bytes.length);
    this.buffer.set(bytes, this.offset);
    this.offset += bytes.length;
  }

  public getBytes(): Uint8Array {
    return this.buffer.subarray(0, this.offset);
  }
}

function writeChunk(fourcc: string, data: Uint8Array): Uint8Array {
  const w = new ByteWriter(data.length + 8);
  w.writeString(fourcc, 4);
  w.writeUint32(data.length);
  w.writeBytes(data);
  // Padding para alinhar em palavra de 2 bytes
  if (data.length % 2 !== 0) {
    w.writeUint8(0);
  }
  return w.getBytes();
}

function writeList(type: string, chunks: Uint8Array[]): Uint8Array {
  let innerLen = 4;
  for (const c of chunks) innerLen += c.length;

  const w = new ByteWriter(innerLen + 8);
  w.writeString('LIST', 4);
  w.writeUint32(innerLen);
  w.writeString(type, 4);
  for (const c of chunks) {
    w.writeBytes(c);
  }
  if (innerLen % 2 !== 0) {
    w.writeUint8(0);
  }
  return w.getBytes();
}

/**
 * Constrói um arquivo SoundFont 2.04 (.sf2) a partir de descritores de amostras.
 */
export function buildSoundFont2File(presetName: string, samples: Sf2SampleDescriptor[]): Uint8Array {
  // 1. INFO LIST
  const ifilData = new ByteWriter(4);
  ifilData.writeUint16(2); // Major
  ifilData.writeUint16(4); // Minor
  const ifilChunk = writeChunk('ifil', ifilData.getBytes());

  const isngData = new ByteWriter(16);
  isngData.writeString('EMU8000\0');
  const isngChunk = writeChunk('isng', isngData.getBytes());

  const inamData = new ByteWriter(presetName.length + 1);
  inamData.writeString(presetName + '\0');
  const inamChunk = writeChunk('INAM', inamData.getBytes());

  const infoList = writeList('INFO', [ifilChunk, isngChunk, inamChunk]);

  // 2. SDTA LIST (Dados de amostra PCM concatenados)
  const smplWriter = new ByteWriter();
  const sampleOffsets: Array<{ start: number; end: number; loopStart: number; loopEnd: number }> = [];

  for (const s of samples) {
    const startIdx = smplWriter.getBytes().length / 2;
    // Escreve as amostras PCM de 16 bits
    for (let i = 0; i < s.pcm.length; i++) {
      smplWriter.writeInt16(s.pcm[i]);
    }
    const endIdx = smplWriter.getBytes().length / 2;
    // Escreve 46 amostras de silêncio (padding zero da especificação SoundFont)
    for (let i = 0; i < 46; i++) {
      smplWriter.writeInt16(0);
    }
    sampleOffsets.push({
      start: startIdx,
      end: endIdx,
      loopStart: startIdx,
      loopEnd: endIdx
    });
  }

  const smplChunk = writeChunk('smpl', smplWriter.getBytes());
  const sdtaList = writeList('sdta', [smplChunk]);

  // 3. PDTA LIST (Cabeçalhos e geradores)
  // phdr: 1 preset + 1 terminal EOP (38 bytes cada)
  const phdr = new ByteWriter();
  phdr.writeString((presetName || 'Preset').slice(0, 19) + '\0', 20);
  phdr.writeUint16(0); // wPreset = 0
  phdr.writeUint16(0); // wBank = 0
  phdr.writeUint16(0); // wPresetBagNdx = 0
  phdr.writeUint32(0); // dwLibrary
  phdr.writeUint32(0); // dwGenre
  phdr.writeUint32(0); // dwMorphology
  // Terminal EOP
  phdr.writeString('EOP\0', 20);
  phdr.writeUint16(255);
  phdr.writeUint16(255);
  phdr.writeUint16(1); // Aponta para fim do bag
  phdr.writeUint32(0);
  phdr.writeUint32(0);
  phdr.writeUint32(0);

  // pbag: 1 bag + 1 terminal
  const pbag = new ByteWriter();
  pbag.writeUint16(0); // wGenNdx = 0
  pbag.writeUint16(0); // wModNdx = 0
  pbag.writeUint16(1); // Terminal
  pbag.writeUint16(0);

  // pmod: terminal vazio
  const pmod = new ByteWriter(10);
  for (let i = 0; i < 10; i++) pmod.writeUint8(0);

  // pgen: 1 gerador apontando para instrumento 0 + terminal
  const pgen = new ByteWriter();
  pgen.writeUint16(41); // gen 41 = instrument
  pgen.writeUint16(0);  // instrument index 0
  pgen.writeUint16(0);  // terminal
  pgen.writeUint16(0);

  // inst: 1 instrumento + 1 terminal EOI
  const inst = new ByteWriter();
  inst.writeString('Instrument\0', 20);
  inst.writeUint16(0); // wInstBagNdx = 0
  inst.writeString('EOI\0', 20);
  inst.writeUint16(samples.length); // Terminal

  // ibag: 1 bag por amostra + terminal
  const ibag = new ByteWriter();
  let genAccum = 0;
  for (let i = 0; i < samples.length; i++) {
    ibag.writeUint16(genAccum);
    ibag.writeUint16(0); // wInstModNdx = 0
    genAccum += 3; // 3 geradores por zona: keyRange, overridingRootKey, sampleID
  }
  ibag.writeUint16(genAccum);
  ibag.writeUint16(0);

  // imod: terminal vazio
  const imod = new ByteWriter(10);
  for (let i = 0; i < 10; i++) imod.writeUint8(0);

  // igen: geradores por zona
  const igen = new ByteWriter();
  for (let i = 0; i < samples.length; i++) {
    const s = samples[i];
    // KeyRange (gen 43)
    igen.writeUint16(43);
    igen.writeUint8(s.keyLow);
    igen.writeUint8(s.keyHigh);
    // OverridingRootKey (gen 58)
    igen.writeUint16(58);
    igen.writeUint16(s.rootKey);
    // SampleID (gen 53)
    igen.writeUint16(53);
    igen.writeUint16(i);
  }
  // Terminal
  igen.writeUint16(0);
  igen.writeUint16(0);

  // shdr: 1 cabeçalho por amostra (46 bytes) + terminal EOS
  const shdr = new ByteWriter();
  for (let i = 0; i < samples.length; i++) {
    const s = samples[i];
    const off = sampleOffsets[i];
    shdr.writeString((s.name || `Sample_${i}`).slice(0, 19) + '\0', 20);
    shdr.writeUint32(off.start);
    shdr.writeUint32(off.end);
    shdr.writeUint32(off.loopStart);
    shdr.writeUint32(off.loopEnd);
    shdr.writeUint32(s.sampleRate);
    shdr.writeUint8(s.rootKey);
    shdr.writeUint8(0); // pitchCorrection
    shdr.writeUint16(0); // sampleLink
    shdr.writeUint16(1); // monoSample
  }
  // Terminal EOS (46 bytes)
  shdr.writeString('EOS\0', 20);
  for (let i = 0; i < 26; i++) shdr.writeUint8(0);

  const pdtaList = writeList('pdta', [
    writeChunk('phdr', phdr.getBytes()),
    writeChunk('pbag', pbag.getBytes()),
    writeChunk('pmod', pmod.getBytes()),
    writeChunk('pgen', pgen.getBytes()),
    writeChunk('inst', inst.getBytes()),
    writeChunk('ibag', ibag.getBytes()),
    writeChunk('imod', imod.getBytes()),
    writeChunk('igen', igen.getBytes()),
    writeChunk('shdr', shdr.getBytes())
  ]);

  // 4. Arquivo RIFF Total
  const riffContentLen = 4 + infoList.length + sdtaList.length + pdtaList.length;
  const riff = new ByteWriter(riffContentLen + 8);
  riff.writeString('RIFF', 4);
  riff.writeUint32(riffContentLen);
  riff.writeString('sfbk', 4);
  riff.writeBytes(infoList);
  riff.writeBytes(sdtaList);
  riff.writeBytes(pdtaList);

  return riff.getBytes();
}

/**
 * Sintetiza 5 oitavas (C2 a C6) e exporta um arquivo SoundFont 2 multisample.
 */
export function exportMultisampleSoundFont(
  presetName: string,
  synthFn: (freqHz: number, durationSec: number) => Int16Array
): Uint8Array {
  const notes = [
    { name: 'C2', rootKey: 36, freq: 65.41,  keyLow: 0,  keyHigh: 41 },
    { name: 'C3', rootKey: 48, freq: 130.81, keyLow: 42, keyHigh: 53 },
    { name: 'C4', rootKey: 60, freq: 261.63, keyLow: 54, keyHigh: 65 },
    { name: 'C5', rootKey: 72, freq: 523.25, keyLow: 66, keyHigh: 77 },
    { name: 'C6', rootKey: 84, freq: 1046.50, keyLow: 78, keyHigh: 127 }
  ];

  const descriptors: Sf2SampleDescriptor[] = [];
  for (const n of notes) {
    const pcm = synthFn(n.freq, 0.8); // 800 ms por amostra
    descriptors.push({
      name: `${presetName}_${n.name}`,
      rootKey: n.rootKey,
      keyLow: n.keyLow,
      keyHigh: n.keyHigh,
      pcm,
      sampleRate: 48000
    });
  }

  return buildSoundFont2File(presetName, descriptors);
}

/**
 * Dispara o download de um buffer binário no navegador.
 */
export function triggerBinaryDownload(buffer: Uint8Array, filename: string, mimeType = 'application/octet-stream'): void {
  if (typeof window === 'undefined') return;
  const blob = new Blob([buffer as any], { type: mimeType });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}
