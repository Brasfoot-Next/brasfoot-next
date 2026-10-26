# Brasfoot Next

**Brasfoot Next** é uma reescrita em Rust do Brasfoot 22-23, ampliada com novos sistemas de gestão. A base é o mesmo jogo: o mesmo motor de partida, as mesmas regras, as mesmas telas, só que com visual moderno, código legível e rodando no desktop e no navegador.

> Projeto de fã, sem vínculo com os autores do Brasfoot original.

## O que é

- **Mesmo jogo, cara nova.** O motor de partida, o calendário, as competições, o mercado e a evolução dos jogadores são portados com fidelidade, inclusive as regras estranhas e os bugs que afetam a jogabilidade.
- **Muito mais gestão.** Táticas, premiações, Hall da Fama, jornal, economia, janelas de transferência, empréstimos, categorias de base, olheiros, diretoria, patrocínio, torcida, investidores, Super Mundial, estádios, comissão técnica e configurações fazem parte do jogo.
- **Patches pelo GitHub.** Times, elencos e ligas ficam em pacotes abertos (TOML, CSV e PNG). O jogo baixa o [banco oficial](https://github.com/Brasfoot-Next/database) e qualquer patch pelo link do repositório, empilha um sobre o outro e avisa quando há versão nova.
- **Importa sua carreira.** Os saves originais (`.s22`) são lidos diretamente e migrados uma vez para o formato próprio do Brasfoot Next (`.bfn`).
- **Pronto para tradução.** Todo texto da interface fica em catálogos [Fluent](https://projectfluent.org/) em `locales/`. Hoje só existe `pt-BR`.
- **Validação diferencial.** Os testes comparam o resultado do Rust com o do jogo original rodando com a mesma semente, com uma réplica exata do `java.util.Random`.

## Estrutura

| Pasta | Conteúdo |
|---|---|
| `crates/model` | Tipos de dados do jogo: jogador, clube, técnico, competição, calendário, partida |
| `crates/engine` | Motor de partida, táticas e a réplica do RNG do Java |
| `crates/simulation` | O loop do jogo sem interface: rodadas, competições, clubes da IA, mercado, envelhecimento e evolução |
| `crates/systems` | Sistemas de gestão: premiações, economia, base, staff, torcida, estádios e os demais |
| `crates/persistence` | Formato de save nativo (`.bfn`) e importador dos saves originais (`.s22`) |
| `crates/datapack` | Pacotes de dados (times, elencos, ligas, imagens), camadas de patches e a ferramenta `brasfoot-data` |
| `crates/i18n` | Tradutor (Fluent) |
| `crates/app` | O aplicativo (eframe/egui), binário `brasfoot-next` |
| `tools/` | Ferramentas de apoio: exportador Java dos dados originais, geradores de código |
| `locales/` | Catálogos de texto |
| `docs/` | Formato de save, patches e motor de partida |

## Rodando

Precisa do Rust estável (edição 2024).

```sh
cargo run -p brasfoot-next      # abre o jogo
cargo test --workspace          # roda os testes
```

Versão web (WebAssembly) com o [Trunk](https://trunkrs.dev/):

```sh
rustup target add wasm32-unknown-unknown
trunk serve                     # http://localhost:8080
```

Times, elencos e ligas vêm da tela **Patches**, na abertura do jogo. O [banco oficial](https://github.com/Brasfoot-Next/database) já vem cadastrado e é baixado na primeira vez. Saves `.s22` importados continuam usando os escudos do Brasfoot instalado.

## Documentação

- [`docs/save.md`](docs/save.md): o formato `.bfn` e o leitor de `.s22`.
- [`docs/patches.md`](docs/patches.md): o formato dos pacotes de times, elencos e ligas, e como montar um patch.
- [`docs/motor.md`](docs/motor.md): o motor de partida e como ele é validado contra o original.
- [`tools/exporter/README.md`](tools/exporter/README.md): o exportador que gera as referências dos testes diferenciais.

## Licença

O código está sob a [GPL-3.0](LICENSE). Os dados do banco oficial têm licença própria, no [repositório deles](https://github.com/Brasfoot-Next/database).
