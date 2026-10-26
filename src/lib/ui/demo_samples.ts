export type DemoSampleId = 'voice' | 'synth' | 'car_horn';

export interface DemoSample {
  id: DemoSampleId;
  label: string;
  category: string;
  icon: string;
  description: string;
  url: string;
}

export const DEMO_SAMPLES: DemoSample[] = [
  {
    id: 'voice',
    label: 'Voz Falada',
    category: 'Vocal / Acústico',
    icon: '🗣️',
    description: 'Voz humana natural com formantes acústicos e curvas contínuas de pitch.',
    url: './demos/voice.wav'
  },
  {
    id: 'synth',
    label: 'Sintetizador',
    category: 'Harmônico / Eletrônico',
    icon: '🎹',
    description: 'Timbre eletrônico brilhante com sobretons bem definidos e modulação harmônica.',
    url: './demos/synth.wav'
  },
  {
    id: 'car_horn',
    label: 'Buzina Acústica',
    category: 'Denso / Polifônico',
    icon: '🎺',
    description: 'Acorde denso com múltiplas frequências fundamentais e batimentos acústicos.',
    url: './demos/car-horn.wav'
  }
];

export function getDemoSampleMetadata(id: DemoSampleId): DemoSample {
  const sample = DEMO_SAMPLES.find(s => s.id === id);
  if (!sample) {
    throw new Error(`Demonstração não encontrada para o ID: ${id}`);
  }
  return sample;
}

/**
 * Carrega os bytes do arquivo WAV da demonstração de forma assíncrona.
 */
export async function loadDemoSampleBytes(
  id: DemoSampleId,
  fetchFn: typeof fetch = fetch
): Promise<Uint8Array> {
  const metadata = getDemoSampleMetadata(id);
  const response = await fetchFn(metadata.url);
  if (!response.ok) {
    throw new Error(`Falha ao carregar amostra de áudio [${id}]: status ${response.status}`);
  }
  const arrayBuffer = await response.arrayBuffer();
  return new Uint8Array(arrayBuffer);
}
