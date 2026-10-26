# Saves

## Formato nativo: `<nome>.bfn`

Um arquivo só por carreira. No jogo original são o `.s22`, o `.sbck`, o `.info` e uns 30 arquivos dos sistemas de gestão (`<save>.caixa`, `<save>.janelas`...); aqui tudo vai junto.

O `.bfn` é um fluxo zstd com dois frames:

1. **Cabeçalho**: um *skippable frame* do zstd (magic `0x184D2A50`) com `BFNX` e os metadados em JSON (`SaveMeta`: versão do formato, versão do app, clube, técnico, data, temporada, hora do save, save original de onde veio). A tela de carregar lê só isso.
2. **Conteúdo**: um frame zstd normal, com checksum, com o estado inteiro em CBOR (RFC 8949). É autodescritivo como JSON (campos novos entram com `#[serde(default)]`) e guarda qualquer `double` do Java exatamente, inclusive `NaN`, que JSON não representa.

O cabeçalho é JSON legível; o conteúdo sai com `zstdcat <save>.bfn` e qualquer leitor de CBOR.

Garantias:
- **Gravação atômica**: escreve em `<nome>.bfn.tmp`, faz `fsync` e só então renomeia. Um travamento no meio do save nunca corrompe a carreira.
- **Backups**: as 3 versões anteriores ficam em `.bak1` (a mais nova) até `.bak3`. Substitui o `.sbck` do original.
- **Versões**: `FORMAT_VERSION` sobe quando o conteúdo muda de um jeito que `#[serde(default)]` não resolve. Cada subida ganha uma `Migration` que atualiza o JSON antigo antes de ler. Save de versão mais nova que o app é recusado com erro claro.

Código: `crates/persistence/src/native.rs`.

Versões do formato (migrações em `crates/persistence/src/migrations.rs`):
- **1**: primeira versão.
- **2**: guarda os números de save dos jogadores (`Player::save_number`, `save_list`) e das transferências (`Transfer::player_number`, `player_list`). Um save 1 recebe os números que o próximo save daria.
- **3**: nomeia os campos das seleções e das eliminatórias (`world_cup_season`, `south_american_qualifiers`, ...), corrige nomes das primeiras importações (Finalíssima, Sul-Americana, obras de estádio, Liga Europa e Conference trocadas, campos do Mundial de Clubes) e guarda o número de competição de cada registro de estatística (`H.ft`, `competition_number`) ao lado do vínculo (`competition_id`). Um save 2 recebe o número da competição ligada.

### Números de save e histórico de transferências

No original o jogador de cada transferência (`ap.U`) é `transient`. Ao salvar, `g.b.Ai()` numera os jogadores de `c.a.SR.O()` e depois os de `c.a.SR.Q()` numa sequência só (`F.ei`, com a lista em `F.ej`), e cada transferência grava o número do seu jogador (`ap.mb()`). Ao carregar, `g.b.Aj()` procura em cada transferência o primeiro jogador da lista com aquele número (`ap.me()`). Um jogador que saiu das listas (aposentado) não é renumerado: fica com o número do save anterior, que outro jogador pode ter agora, e depois do load a transferência dele passa a mostrar esse outro. Um jogador criado depois do último save fica com -1 e a transferência dele fica sem jogador. É um bug do original e fica assim.

O port faz o mesmo: `Career::save` chama `World::number_players_for_save()` e `Career::open` chama `World::relink_transfers()` (`crates/model/src/save_numbers.rs`). Os testes diferenciais passam o mundo pelo mesmo ciclo antes de comparar, porque o harness salva e o exportador carrega.

## Compatibilidade com o Brasfoot original

### `.s22` (Kryo)

O jogo grava com `new Kryo()` padrão (`c.a`, métodos de salvar e `c(String, boolean)`): duas raízes, `c.a.SR` (`best.f`, o estado do jogo) e `c.a.afQ` (`d.ay`). O Rust lê o `.s22` direto, sem Java:

- `crates/persistence/data/kryo-schema.json`: para cada classe do jogo, o serializador que o Kryo escolhe e os campos na ordem exata do fio. Gerado do próprio jar por `tools/exporter/export.sh schema` (classe `exporter.KryoSchema`), com as classes dos sistemas de gestão na frente, como no jar estendido (`bf22-23-mod.jar`).
- `crates/persistence/src/kryo/`: porte do lado de leitura do Kryo 4.0.2 (varints, strings, referências, nomes de classe, FieldSerializer, coleções, mapas, arrays, Calendar). Produz um grafo genérico (`Graph`).

**Validação diferencial**: `export.sh fixture` monta um save fictício com as próprias classes do jogo (clubes, jogadores e técnicos inventados, valores aleatórios com semente fixa, objetos compartilhados, ciclos, nulos, listas mistas, arrays, datas e strings em todas as codificações do Kryo), grava com o Kryo do jogo e escreve o dump canônico dele pelo lado Java. O teste `kryo_differential` exige que o leitor Rust produza o mesmo dump, byte a byte. O fixture fica em `crates/persistence/tests/fixtures/` e o teste roda em todo `cargo test`. Nenhum save de jogador é usado em teste.

Campos `transient` não vão no fio; o schema lista quais são, porque ficam com o valor do construtor sem argumentos.

### Arquivos dos sistemas de gestão (`<save>.<ext>`)

Texto, uma linha por registro, campos separados por tab. Cada um é lido pelo sistema correspondente quando ele for portado (`crates/systems`), a partir do código original que o grava.

### Do grafo para o modelo

`crates/persistence/src/import/` converte o `Graph` no `World` de `crates/model`: uma tabela (arena) por classe compartilhada (jogadores, clubes, técnicos, estádios, partidas, competições, fases, países, estaduais) e ids no lugar das referências Java. O código das structs e do conversor é gerado por `tools/modelgen/gen_model.py` a partir do schema e do dicionário `tools/modelgen/names.py` (o ofuscador usou um dicionário global: o mesmo nome ofuscado significa a mesma coisa em todas as classes). A tabela completa está em `docs/mapa/fields.md`.

Teste: `import_fictional` importa o save fictício, confere que cada objeto caiu na sua tabela e que o `World` sobrevive ida e volta pelo `.bfn`.

### Ainda não feito

- `.ban` dos times e `conf_*` (serialização Java padrão, que descreve os próprios campos e também dá pra ler em Rust).
- Exportar de volta para `.s22`, para continuar a carreira no jogo original. O schema já permite escrever no formato do Kryo; falta o caminho do modelo de volta para o grafo.
