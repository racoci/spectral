# Especificação: Surprise Encoding Bayesiano & Lookahead Causal (E58-E59)

## 1. Princípio de Informação e Surpresa
Em áudio musical real (como notas sustentadas de violoncelo, flauta ou sintetizadores), o comportamento de frequência e amplitude é frequentemente altamente previsível. Transmitir parâmetros a cada quadro de 5 ms consome largura de banda desnecessária.

O **Surprise Encoding** introduz um limiar de informação $\lambda$:
$$\Delta G = \frac{|f_{\text{observado}} - f_{\text{predito}}|^2}{2\sigma^2}$$

## 2. Regra de Decisão do Transmissor
- Se $\Delta G \le \lambda$ e o tempo sem transmissão for menor que o intervalo de *heartbeat* (15 quadros):
  - **Decisão: Pular Transmissão (Skip)**.
  - O decodificador propaga a trajetória livremente através de sua inércia causal ($\hat{f} = f + \text{slope}$).
- Se $\Delta G > \lambda$ (ocorreu uma transição rápida, ataque ou pitch bend):
  - **Decisão: Transmitir Inovação**.
  - O codificador envia a correção de frequência e momento, sincronizando o decodificador instantaneamente.

## 3. Resultados Experimentais
- Em trechos sustentados, **73.3% das transmissões são suprimidas**.
- O erro médio no decodificador permanece abaixo de **1.11 cents**, totalmente inaudível ao ouvido humano.
