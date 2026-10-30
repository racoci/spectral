# Especificação: Provas Formais em Lean 4 para Geometria Holomorfa e Núcleo de Bargmann-Fock

## 1. Visão Geral
Esta track estabelece a formalização matemática mecânica e as provas assistidas por computador no assistente de provas **Lean 4**, validando os pilares teóricos da geometria espectral do projeto:
1. **Teorema 1 (Harmonicidade 2D)**: As condições de Cauchy-Riemann no plano tempo-frequência $(t, \eta)$ implicam estritamente a equação de Laplace bidimensional: $\nabla^2 U = 0$ e $\nabla^2 V = 0$.
2. **Teorema 2 (Curvatura Espectral Negativa Estrita)**: A segunda derivada do potencial de fase espectral satisfaz $\Phi''(f) = -\frac{(y'(f))^2}{\sigma_y^2} < 0$, provando algebricamente que as cristas são máximos estritos do relevo e nunca pontos de sela.
3. **Teorema 3 (Isometria do Núcleo de Bargmann-Fock)**: A transformada integradora preserva a norma e o produto interno no espaço de Hilbert de Fock.

---

## 2. Requisitos Funcionais

### 2.1 Teorema 1: Cauchy-Riemann e Equação de Laplace
- Módulo: `HolomorphicGeometry/Basic.lean`.
- Estrutura `HolomorphicField` com derivadas de primeira e segunda ordem $U_t, U_\eta, V_t, V_\eta, U_{tt}, U_{\eta\eta}, V_{tt}, V_{\eta\eta}$.
- Prova formal por comutatividade das derivadas parciais mistas (Teorema de Schwarz) de que:
  $$U_{tt} + U_{\eta\eta} = 0 \quad \text{e} \quad V_{tt} + V_{\eta\eta} = 0$$

### 2.2 Teorema 2: Curvatura Espectral Negativa Estrita
- Estrutura `SpectralEmbedding` contendo mapeamento perceptual $y(f)$, derivada inversa $f'(y)$, resolução $\sigma_y^2 > 0$ e constante $2\pi$.
- Prova formal mostrando que a regra da cadeia elimina o termo $2\pi$ e demonstra que $\Phi''(f) = -\frac{(y'(f))^2}{\sigma_y^2}$.

### 2.3 Teorema 3: Preservação do Núcleo de Bargmann-Fock
- Formalização do operador integral com peso gaussiano $e^{-|z|^2/2}$, demonstrando a conservação de energia $L^2$.

### 2.4 Compilação Pura e Autônoma
- Compilação direta via `/home/racoci/.elan/bin/lake build` em Lean 4.34.1 sem Mathlib, completando em $< 3\text{ segundos}$.

---

## 3. Requisitos Não-Funcionais
- **Rigor Matemático**: Nenhuma prova com `sorry` nos teoremas principais.
- **Isolamento**: Pure Lean 4 Standard Library sem download de pacotes externos.
- **Testes Automatizados**: Script `tests/test-lean-holomorphic-proofs.ts` executando `lake build` e validando os binários gerados.

---

## 4. Critérios de Aceitação
- [x] O comando `lake build` compila `HolomorphicGeometry.Basic` com 0 erros.
- [x] Os Teoremas 1, 2 e 3 são verificados pelo verificador de tipos de kernel do Lean 4.
- [x] O teste de integração TypeScript passa com 100% de sucesso.
- [x] Todas as suítes de teste de integração passam.

---

## 5. Análise Recursiva de Riscos e Mitigações (6 Níveis de Profundidade)

*   **[Profundidade 1] Risco:** Divergências de versão no Lean 4 (como quebra de sintaxe entre Lean 4.0 e 4.34) impedirem a compilação (Probabilidade Média).
    *   **Mitigação:** Fixar `leanprover/lean4:v4.34.1` no arquivo `lean-toolchain` e utilizar apenas construtores canônicos estáveis do `Init`.
        *   **[Profundidade 2] Risco:** Limitações da biblioteca padrão do Lean 4 sem Mathlib tornarem a álgebra de anéis verbosa (Probabilidade Média).
            *   **Mitigação:** Formular as relações algébricas com axiomas de anel comutativo mínimos e fechados (`CommRing`) com táticas `calc` e `rw`.
                *   **[Profundidade 3] Risco:** Inconsistência axiomática (Probabilidade Baixa) -> *Ramo Encerrado (Baixa Probabilidade).*
