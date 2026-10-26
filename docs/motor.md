# Motor de partida

Porte de `c.b` na versão estendida (`mod/patch/c/b.java`), das táticas (`bfmod.Taticas`) e da partida da IA de `best.I` (`V()` e o que ela chama). Código em `crates/engine`.

| Módulo | Porte de |
|---|---|
| `engine.rs` | `c.b`: lance a lance, duelo de meio-campo, ataque contra defesa, chute contra goleiro, gol (tipo, artilheiro, assistência, gol contra, batedores) |
| `simulate.rs` | `I.V()`, `I.a(I, int, int)` (cartões, lesões), eventos `I.a(int, int, I, ah, F, F, int, int)`, substituições, mudanças da IA `I.m`, pênaltis de mata-mata, `I.hi()` |
| `tactics.rs` | `bfmod.Taticas`: efeitos, escolha da IA, previsão analítica e conselho do analista |
| `value.rs` | `c.b.B` (valor do jogador no motor), `F.gF` (fora de posição), `bfmod.Motor.valor` |
| `lineup.rs` | escalação da IA `ah.a(ah, I, int, int, boolean)`, `ah.a(ArrayList, int, boolean, boolean)`, `F.a(I, boolean)` |
| `stadium.rs` | público e renda `v.b(I)`, preços padrão `v.a(int, ah)` |
| `injury.rs` | lesão `F.p(ah)` |
| `events.rs` | estatísticas do jogador `F.a(int, ah, double, at)`, `F.h(at)` |
| `clubs.rs` | batedores e capitão (`ah.kd`, `ke`, `lq`, `lr`) |
| `hooks.rs` | onde os sistemas de gestão entram na partida (satélites, patrimônio, comissão, configurações) |

## Validação diferencial

`tools/exporter/export.sh engine`:

1. Compila um `java.util.Random` com o construtor sem semente trocado por um contador (`tools/exporter/random-patch`, carregado com `--patch-module java.base`).
2. `exporter.EngineHarness` monta um mundo fictício com as classes do jogo (8 clubes inventados, 26 jogadores cada, técnicos, estádios, 8 competições de tipos diferentes, alguns suspensos e lesionados), grava como o jogo (`g.b.Ag`, `Ai`, Kryo), carrega como o jogo (Kryo, `g.b.Ah`, `Af`, `Aj`).
3. Zera o contador, recria os `Random` estáticos numa ordem fixa (`aq.tI`, `Taticas.SORTE`, `Collections.r`, listas estáticas de `best.I`) e joga as 32 partidas do dia: `I.hi()`, escalação da IA dos dois lados, `I.V()`.
4. Escreve o resultado (eventos, estatísticas, escalações, táticas, e o estado de cada jogador) em `crates/engine/tests/fixtures/engine-<táticas>.txt`.

O teste `crates/engine/tests/differential.rs` importa o mesmo mundo, cria o `Runtime` com o mesmo contador, joga o mesmo dia e exige o mesmo texto. Passa com e sem as táticas.

Lições do caminho, que valem para os próximos diferenciais:
- Toda classe com `Random` estático consome uma semente quando é inicializada. O harness inicializa todas antes de ligar o contador (`STATIC_RANDOMS`).
- O mundo também é montado com o contador ligado, senão cada execução gera outro mundo.
- Objetos novos usam os inicializadores de campo do Java (`dz = -1`, condição 100...): o gerador do modelo produz `Default` a partir deles.
- As táticas guardam a lista viva de jogadores em campo da partida, não uma cópia.

## Bugs do original mantidos

- `F.fU()` (força efetiva): divisão inteira, só jogador com 100 de condição conta.
- Gol normal soma duas vezes no `components.s` do artilheiro; no pênalti, o gol conta para o jogador sorteado, não para o batedor.
- No erro de escalação vazia, o motor escala o visitante nas listas do mandante.
- Substituição aleatória da IA consulta a lista de quem entrou do mandante também para o visitante.
- O peso de "chute para fora" é sobrescrito pelo de defesa quando não é campo neutro.
- Na assistência, o traço 13 pesa 1 no total e 2 no sorteio.
