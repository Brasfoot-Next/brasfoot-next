# Patches

Times, elencos, ligas e imagens do Brasfoot Next ficam em **pacotes de dados**: pastas de arquivos de texto (TOML e CSV) e PNG, versionadas no git. Eles substituem os `teams/*.ban`, `conf_ligas_nacionais/*.cfg` e `conf_estadual/*.ces` do jogo original.

Um pacote é uma **base** (um banco completo) ou um **patch** (mudanças sobre uma base). No jogo, a tela Patches empilha as fontes: a base embaixo e os patches por cima, na ordem escolhida. Os dados só entram no **Novo jogo**, e carreiras em andamento nunca mudam.

Código: `crates/datapack`. Ferramenta de linha de comando: `brasfoot-data`.

## Estrutura

```text
manifest.toml
clubs/<país>/<clube>/club.toml
                     squad.csv
                     youth.csv        (opcional: juniores)
                     crest.png        (escudo)
                     kit.png          (camisa 1; kit2.png e kit3.png opcionais)
leagues/<país>.toml
state-leagues/<estado>.toml
national-teams/<país>/crest.png, kit.png, kit2.png
trophies/<tipo>/<nome>.png
```

- **País**: trigrama da FIFA (`BRA`, `GER`, `ENG`, `SCO`, `NED`...). Nas pastas fica em minúsculas (`clubs/ger/`). Países fora da FIFA usam o código da confederação (`GLP`, `BOE`) ou, sem ele, o ISO 3166 (`MCO`, `FSM`). A tabela completa está em `crates/datapack/src/codes.rs`.
- **Estado**: a sigla (`SP`, `RJ`), só para clubes do Brasil.
- **Clube**: o id é a pasta, `<país>/<clube>`, por exemplo `bra/vila-exemplo`. Usa só letras minúsculas sem acento, números e hífen. O id não muda quando o nome do clube muda.
- README, LICENSE e outros `.md`/`.txt` na raiz são ignorados. Qualquer outro arquivo fora da estrutura gera um aviso.

## manifest.toml

```toml
format = 1                      # versão do formato
id = "fulano/liga-exemplo"      # dono/nome, normalmente o repositório do GitHub
kind = "patch"                  # "base" ou "patch"
name = "Liga Exemplo 2026"
description = "Times e elencos da Liga Exemplo"   # opcional
version = "2026.1"                                # opcional, texto livre
extends = "brasfoot-next/database"                # opcional: a base pensada para o patch

[remove]                        # só em patches
clubs = ["bra/clube-que-saiu"]
leagues = ["ARG"]               # volta ao formato padrão do jogo
state_leagues = ["MG"]
```

## Camadas

Cada arquivo vem da camada mais alta que o tem. Um patch não precisa repetir o clube inteiro:

- só `clubs/bra/vila-exemplo/squad.csv`: troca o elenco e mantém o resto;
- só `crest.png`: troca o escudo;
- uma pasta nova com `club.toml` e `squad.csv`: adiciona um clube;
- `[remove]` no manifest: tira clubes, ligas ou estaduais das camadas de baixo.

## club.toml

```toml
name = "Vila Exemplo"
country = "BRA"
state = "SP"                    # só clubes do Brasil
level = 12                      # 1 a 25; fora disso o jogo usa 10
reputation = 3
colors = ["#112233", "#ffffff"]
rivals = ["bra/uniao-ficticia"] # clássicos; basta um dos dois clubes listar o outro

[stadium]
name = "Estádio Faz de Conta"
capacity = 25000                # 1000 a 120000; fora disso o jogo usa 10000

[coach]
name = "Técnico Inventado"
country = "BRA"
```

## squad.csv e youth.csv

Uma linha por jogador, com cabeçalho. Abre e salva em qualquer planilha (UTF-8).

```csv
name,position,side,age,country,potential,starter,star,world_class,trait1,trait2
João Exemplo,GK,R,29,BRA,7,true,false,false,reflexes,penalty-saving
,FW,L,22,ARG,9,false,true,false,finishing,pace
```

| Coluna | Valores |
|---|---|
| `name` | vazio: o jogo sorteia um nome do país do jogador |
| `position` | `GK` goleiro, `FB` lateral, `CB` zagueiro, `MF` meia, `FW` atacante |
| `side` | `R` direito, `L` esquerdo |
| `age` | 16 a 48 (juniores 16 a 20); fora disso o jogo usa 35 (juniores 18) |
| `country` | trigrama da FIFA |
| `potential` | 0 a 10 |
| `starter` | `true` se começa como titular |
| `star` | `true` para craque |
| `world_class` | `true` para craque de nível mundial (sempre é craque também) |
| `trait1`, `trait2` | `positioning` colocação, `penalty-saving` defesa de pênalti, `reflexes` reflexo, `rushing-out` saída do gol, `playmaking` armação, `heading` cabeceio, `crossing` cruzamento, `tackling` desarme, `dribbling` drible, `finishing` finalização, `marking` marcação, `passing` passe, `stamina` resistência, `pace` velocidade; vazio para nenhuma |

A força não fica no arquivo: o jogo sorteia no Novo jogo, a partir do clube, do país e das opções, como no original.

## leagues/\<país\>.toml

O formato da liga nacional de um país. Sem o arquivo, o jogo usa o formato padrão. Um país vira liga jogável quando tem times suficientes (10, ou 16 em alguns países).

```toml
country = "ENG"

[[divisions]]
division = 1                    # 1 a 4
name = "Campeonato Inglês"
division_name = "1ª Divisão"
teams = 20
two_rounds = true
relegated = 3
relegated_directly = 2
```

Campos omitidos ficam com o padrão do jogo: `two_rounds = true`, `round_count = 0`, `groups = 0`, `games_within_group = true`, `knockout_teams = 0`, `two_legged_knockout` todos `true` (7 fases), `qualify_by_overall_table = false`, `best_third_places = false`, `tiebreak = 0`, `relegated = 2`, `relegated_directly = 2`, `relegated_by_group = false`, `relegation_playoff = 0`, `two_legged_relegation_playoff = [false, false, false]`, `promotion_places = 0`, `two_legged_promotion_knockout = [false, false, false]`.

## state-leagues/\<estado\>.toml

```toml
state = "SP"

[[divisions]]
division = 1
format = "16-teams-4-groups-sp-2021"
relegated = 2
tiebreak = 0
two_legged_finals = [false, true, true]   # ida e volta em cada uma das três fases finais
```

Formatos: `6-teams`, `8-teams-4-qualify`, `10-teams-4-qualify`, `11-teams-4-qualify`, `12-teams-4-qualify`, `12-teams-8-qualify`, `14-teams-8-qualify`, `16-teams-4-groups-sp-2021`, `16-teams-4-qualify`, `16-teams-8-qualify`, `20-teams-4-groups`.

## Troféus

| Arquivo | Troféu |
|---|---|
| `trophies/league/<país>.png`, `<país>-<divisão>.png`, `default.png` | liga nacional |
| `trophies/cup/<país>.png`, `default.png` | copa nacional |
| `trophies/super-cup/<país>.png`, `default.png` | supercopa |
| `trophies/state/<estado>.png`, `<estado>-<divisão>.png`, `default.png` | estadual |
| `trophies/regional/<nome>.png` | `rio-sao-paulo`, `sul-minas`, `copa-do-nordeste`, `copa-verde` |
| `trophies/continental/<nome>.png` | `libertadores`, `copa-sudamericana`, `recopa-sudamericana`, `champions-league`, `europa-league`, `conference-league`, `european-super-cup`, `intercontinental-cup`, `world-cup`, `copa-america`, `euro`, `africa-cup-of-nations`, `asian-cup`, `gold-cup`, `ofc-nations-cup`, `finalissima`, `nations-league`, `nations-league-c`, `caf-champions-league`, `afc-champions-league`, `concacaf-champions-cup`, `ofc-champions-league` |

## brasfoot-data

```sh
cargo run --release -p brasfoot-datapack --bin brasfoot-data -- <comando>

convert <pasta do jogo> <saída> --id <dono/nome> --name <nome> [--version <v>]
validate <pacote> [--on <base>]...
pack <pacote> <saída> [--on <base>]... [--commit <sha>]
```

- `convert` lê uma instalação do Brasfoot original (sem escrever nela) e gera uma base.
- `validate` confere um pacote. Um patch sozinho é conferido arquivo por arquivo; com `--on`, sobre a base, como o jogo vai ler. Erros impedem o uso; avisos mostram valores que o jogo corrige sozinho.
- `pack` valida e corta o pacote em pedaços para download: `index.json` com o hash de cada pedaço, `data.zip` com os textos e as imagens agrupadas (`clubs-<letra>.zip`, `national-teams.zip`, `trophies.zip`). Os pedaços são determinísticos: o que não mudou mantém o hash, e o jogo não baixa de novo.
