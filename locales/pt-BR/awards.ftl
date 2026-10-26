# Season-end awards (bfmod.Premios, Temporada, Tecnicos, HallDaFama, Clubes)

# Premios.Tipo: name, and the counts of the player card (Premios.resumo)
award-kind-ballon-dor = Bola de Ouro
award-kind-golden-shoe = Chuteira de Ouro
award-kind-golden-glove = Luva de Ouro
award-kind-golden-boy = Golden Boy
award-kind-team-of-the-year = Seleção do Ano
award-kind-coach-of-the-year = Técnico do Ano
award-count-ballon-dor = { $count } { $count ->
    [one] Bola
   *[other] Bolas
}
award-count-golden-shoe = { $count } { $count ->
    [one] Chuteira
   *[other] Chuteiras
}
award-count-golden-glove = { $count } { $count ->
    [one] Luva
   *[other] Luvas
}
award-count-golden-boy = { $count } { $count ->
    [one] Golden Boy
   *[other] Golden Boys
}
award-count-team-of-the-year = { $count } { $count ->
    [one] Seleção
   *[other] Seleções
}
award-count-coach-of-the-year = { $count } { $count ->
    [one] Técnico do Ano
   *[other] Técnicos do Ano
}

# Temporada.titulos
award-title-champion = Campeão { $competition }
award-title-runner-up = Vice { $competition }

# Temporada.selecao
award-slot-goalkeeper = Goleiro
award-slot-right-back = Lateral Direito
award-slot-centre-back = Zagueiro
award-slot-left-back = Lateral Esquerdo
award-slot-midfielder = Meia
award-slot-forward = Atacante

# Tecnicos.Candidato.resumo
award-coach-campaign = { $position }º no { $league } (previsto { $expected }º)

# Clubes.artilheiros
award-club-top-scorer = { $name } ({ $goals })

# HallDaFama.Recorde
record-goals = Mais gols numa temporada
record-assists = Mais assistências numa temporada
record-contributions = Mais gols + assistências numa temporada
record-rating = Maior nota média (mín. 20 jogos)

# HallDaFama.situacoes
hall-retired-in = Aposentado em { $year }
hall-retired = Aposentado
hall-free-agent = Sem clube

# The awards window (bfmod.CerimoniaDialog)
awards-ceremony-window-title = Prêmios da temporada
awards-ceremony-button-continue = Continuar
awards-ceremony-button-close = Fechar
awards-ceremony-header-title = BOLA DE OURO { $year }
awards-ceremony-header-announce = O melhor jogador do mundo na temporada é
awards-ceremony-header-leader = Líder do ranking mundial da temporada
awards-ceremony-header-nobody = Nenhum jogador com jogos suficientes nesta temporada.
awards-ceremony-header-club-line = { $club }   ·   { $position }   ·   { $age } anos
awards-ceremony-header-stats = { $games } jogos   ·   { $goals } gols   ·   { $assists } assistências   ·   nota média { $rating }
awards-ceremony-header-golden-shoe = Chuteira: { $player } ({ $goals } gols)
awards-ceremony-header-golden-glove = Luva: { $player }
awards-ceremony-header-golden-boy = Golden Boy: { $player }
awards-ceremony-header-coach = Técnico: { $name }
awards-ceremony-no-club = Sem clube
awards-ceremony-tab-ballon-dor = Bola de Ouro
awards-ceremony-tab-golden-shoe = Chuteira
awards-ceremony-tab-golden-glove = Luva
awards-ceremony-tab-golden-boy = Golden Boy
awards-ceremony-tab-coach = Técnico
awards-ceremony-tab-team-of-year = Seleção do Ano
awards-ceremony-tab-winners = Premiados
awards-ceremony-tab-clubs = Clubes
awards-ceremony-tab-hall-of-fame = Hall da Fama
awards-ceremony-tab-super-world-cup = Super Mundial
awards-ceremony-column-rank = #
awards-ceremony-column-player = Jogador
awards-ceremony-column-position = Pos
awards-ceremony-column-age = Idade
awards-ceremony-column-club = Clube
awards-ceremony-column-games-short = J
awards-ceremony-column-goals-short = G
awards-ceremony-column-assists-short = A
awards-ceremony-column-rating = Nota
awards-ceremony-column-points = Pontos
awards-ceremony-column-league-weight = Peso da liga
awards-ceremony-league-weight = ×{ $weight }
awards-ceremony-column-goals = Gols
awards-ceremony-column-coach = Técnico
awards-ceremony-column-campaign = Campanha
awards-ceremony-column-finish = Pos
awards-ceremony-column-expected = Previsto
awards-ceremony-column-titles = Títulos
awards-ceremony-ordinal = { $n }º
awards-ceremony-coach-unavailable = A classificação desta temporada não está mais disponível.
awards-ceremony-column-year = Ano
awards-ceremony-column-winner-ballon-dor = Bola de Ouro
awards-ceremony-column-winner-golden-shoe = Chuteira de Ouro
awards-ceremony-column-winner-golden-glove = Luva de Ouro
awards-ceremony-column-winner-golden-boy = Golden Boy
awards-ceremony-column-winner-coach = Técnico do Ano
awards-ceremony-winner-cell = { $name }  ({ $club })
awards-ceremony-team-season-label = Temporada:
awards-ceremony-team-partial = { $year } (parcial)
awards-ceremony-clubs-label = Clube:
awards-ceremony-clubs-my-career = Minha carreira de técnico ({ $coach })
awards-ceremony-clubs-empty-history = A história deste clube começa a ser gravada no fim desta temporada.
awards-ceremony-clubs-empty-career = Sua carreira começa a ser gravada no fim desta temporada.
awards-ceremony-column-league = Liga
awards-ceremony-column-top-scorer = Artilheiro
awards-ceremony-idols-scorers = Maiores artilheiros do clube
awards-ceremony-idols-games = Mais jogos pelo clube
awards-ceremony-column-games = Jogos
awards-ceremony-hall-tab-gallery = Galeria de premiados
awards-ceremony-hall-tab-scorers = Artilheiros da história
awards-ceremony-hall-tab-legends = Lendas aposentadas
awards-ceremony-hall-tab-records = Recordes de temporada
awards-ceremony-hall-tab-gems = Joias
awards-ceremony-hall-tab-squads = Elencos mais fortes
awards-ceremony-column-balls = Bolas
awards-ceremony-column-boots = Chuteiras
awards-ceremony-column-gloves = Luvas
awards-ceremony-column-golden-boys = Golden Boys
awards-ceremony-column-teams-of-year = Seleções
awards-ceremony-column-status = Situação
awards-ceremony-column-goals-per-game = Gols/jogo
awards-ceremony-column-retired-year = Aposentou
awards-ceremony-column-last-club = Último clube
awards-ceremony-column-awards = Prêmios
awards-ceremony-legends-empty = Nenhuma lenda se aposentou desde que o mod foi instalado.
awards-ceremony-column-record = Recorde
awards-ceremony-column-mark = Marca
