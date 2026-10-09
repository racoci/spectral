from __future__ import annotations
import numpy as np

class StructuralRouter:
    """
    Roteador Estrutural Exaustivo (E37 - Fase 4).
    Recebe um sinal, divide em blocos, aplica todas as famílias de representação 
    disponíveis (competidores M_k + Raw), mede o custo via ResidualEvaluator
    e seleciona para cada bloco a representação com o menor L_total (Minimum Description Length).
    """
    def __init__(self, block_size: int, evaluator, competitors: list):
        self.block_size = block_size
        self.evaluator = evaluator
        self.competitors = competitors

    def process_signal(self, x: np.ndarray) -> tuple[np.ndarray, list[dict]]:
        """
        Retorna o sinal reconstruído hibridamente e a lista de metadados por bloco (as escolhas do router).
        """
        N = len(x)
        num_blocks = N // self.block_size
        
        x_hat_hybrid = np.zeros(N)
        routing_decisions = []
        
        for i in range(num_blocks):
            start = i * self.block_size
            end = start + self.block_size
            xb = x[start:end]
            
            # Baseline (Raw Audio)
            xb_int = self.evaluator.to_pcm(xb)
            L_raw = self.evaluator.shannon_entropy(xb_int) * self.block_size
            
            best_cost = L_raw
            best_choice = 'RAW_AUDIO'
            best_xb_hat = xb.copy() # Raw não perde fidelidade
            
            # Competidores
            for comp in self.competitors:
                try:
                    theta, xb_hat_comp = comp.fit_and_reconstruct(xb)
                    
                    # Estimate params count
                    num_params = 0
                    if comp.name == 'M1_CQTRidge':
                        num_params = len(theta.get('f', [])) * len(theta.get('f', [[]])[0]) * 3
                    elif comp.name == 'M3_MatrixPencil':
                        num_params = len(theta.get('blocks', [])) * 16
                    elif comp.name == 'M4_HankelSSA':
                        num_params = len(theta.get('U_basis', [])) * len(theta.get('U_basis', [[]])[0]) + len(theta.get('singular_values', []))
                    elif comp.name == 'M6_WaveletMaxima':
                        num_params = theta.get('total_maxima', 0) * 3
                    else:
                        num_params = 50 # Default safe fallback

                        
                    mdl = self.evaluator.evaluate(xb, xb_hat_comp, num_params)
                    
                    if mdl['L_total_bits'] < best_cost:
                        best_cost = mdl['L_total_bits']
                        best_choice = comp.name
                        best_xb_hat = xb_hat_comp
                        
                except Exception as e:
                    pass # Se um extrator falhar num bloco (ex: M3 sem polos), segue em frente
            
            # Salva o bloco
            x_hat_hybrid[start:end] = best_xb_hat
            routing_decisions.append({
                'block': i,
                'method': best_choice,
                'L_bits': best_cost
            })
            
        return x_hat_hybrid, routing_decisions
