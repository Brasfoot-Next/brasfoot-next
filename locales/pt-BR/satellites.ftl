# The multi-club networks (bfmod.Satelites) and the bank (bfmod.Credito)

satellite-policy-young-players = Formar jovens (Sub-23)
satellite-policy-young-players-description = Prioriza escalar e contratar jovens Sub-23 e desenvolver atletas para a matriz.
satellite-policy-promotion = Buscar acesso e títulos
satellite-policy-promotion-description = Investe o próprio caixa de forma agressiva nas janelas para subir de divisão e vencer.
satellite-policy-profit = Gerar lucro para o grupo
satellite-policy-profit-description = Gasto contido no mercado, focando em vender atletas valorizados e acumular caixa.

satellite-role-starter = Titular Absoluto
satellite-role-starter-description = Prioridade máxima de escalação (espera ao menos ~80% dos jogos).
satellite-role-regular = Titular Regular
satellite-role-regular-description = Participação regular em rotação (espera ao menos ~60% dos jogos).
satellite-role-reserve = Reserva
satellite-role-reserve-description = Reserva de desenvolvimento para ganhar rodagem sem exigência rígida.

satellite-event-partnership = Parceria
satellite-event-renewal = Renovação
satellite-event-termination = Rescisão
satellite-event-acquisition = Aquisição
satellite-event-club-sale = Venda de Clube
satellite-event-cash-injection = Injeção de Caixa
satellite-event-profit-withdrawal = Retirada de Lucro
satellite-event-assets = Patrimônio
satellite-event-loan = Empréstimo
satellite-event-return = Retorno
satellite-event-to-parent = Transferência p/ Matriz
satellite-event-definitive = Cessão Definitiva
satellite-event-youth-pull = Captação de Base
satellite-event-sell-on = Mais-Valia (20%)
satellite-event-ai-acquisition = Aquisição IA
satellite-event-ai-partnership = Parceria IA
satellite-event-ai-loan = Empréstimo IA
satellite-event-ai-promotion = Promoção à Matriz
satellite-event-ai-cession = Cessão IA

credit-stage-normal = Normal
credit-stage-normal-description = Clube em dia com suas obrigações.
credit-stage-warning = Advertência
credit-stage-warning-description = Atraso leve: juros de mora e rating rebaixado.
credit-stage-transfer-ban = Transfer Ban
credit-stage-transfer-ban-description = Inadimplência: proibição de contratações e penhora de 30% das receitas.
credit-stage-intervention = Intervenção
credit-stage-intervention-description = Insolvência grave: perda de pontos na liga e leilão compulsório de atletas.

credit-rating-aaa = Solvência excepcional
credit-rating-aa = Grau de investimento muito alto
credit-rating-a = Grau de investimento forte
credit-rating-bbb = Grau de investimento adequado
credit-rating-bb = Grau especulativo / moderado
credit-rating-b = Alta sensibilidade a crises
credit-rating-ccc = Risco substancial de crédito
credit-rating-d = Inadimplente / Risco crítico

# The satellites window (bfmod.SatelitesDialog)
satellites-header = CLUBES SATÉLITES E REDE MULTICLUBES
satellites-no-club = Você não está no comando de um clube no momento.
satellites-subtitle = { $club }  ·  Caixa: { $cash }  ·  Satélites ativos: { $count } ({ $owned } comprados, { $partners } parcerias)
satellites-summary = Anuidades de parceria: { $fees }/ano  ·  Redes ativas no mundo: { $links } vínculos
satellites-tab-mine = Meus Satélites
satellites-tab-search = Procurar Clube
satellites-tab-networks = Redes do Mundo
satellites-button-close = Fechar
satellites-take-a-club = Assuma o comando de um clube para gerir satélites.
satellites-label-satellite = Satélite:
satellites-label-policy = Política:
satellites-label-tactic = Tática:
satellites-check-mirror = Espelhar tática da matriz (+15% evolução)
satellites-label-style = Estilo:
satellites-label-marking = Marcação:
satellites-label-focus = Foco:
satellites-tag-owned = Comprado
satellites-tag-partnership = Parceria
satellites-combo-item = { $club } [{ $tag } · { $league }]
satellites-league-division-long = { $league } ({ $division }ª Div)
satellites-league-division = { $league } ({ $division }ª)
satellites-none = Nenhum clube satélite ativo. Clique em '+ Novo Satélite' ou na aba 'Procurar Clube'.
satellites-status-owned = [CLUBE COMPRADO - 100% DO GRUPO]
satellites-status-partnership = [PARCERIA POR CONTRATO ATÉ { $year }]
satellites-status-suspended = { $link }  ·  ⚠ SUSPENSO (MESMA DIVISÃO)
satellites-status-active = { $link }  ·  Ativo desde { $year }
satellites-finance-owned = Caixa do { $club }: { $cash }  ·  Valor de revenda (90%): { $resale }
satellites-finance-partnership = Caixa estimado: { $cash }  ·  Anuidade: { $fee }/ano  ·  Preço p/ comprar 100%: { $price }
satellites-button-assets = Instalações / Patrimônio...
satellites-button-inject = Injetar Caixa
satellites-button-withdraw = Retirar Lucro (-15%)
satellites-button-buy = Comprar Clube
satellites-button-renew = Renovar (+3 anos)
satellites-button-end = Romper Parceria
satellites-button-sell = Vender Clube (90%)
satellites-button-new = + Novo Satélite
satellites-left-title = Elenco do { $club } e Emprestados no Satélite
satellites-pros-title = Elenco Profissional do { $club } ({ $count } atletas)
satellites-juniors-title = Academia de Base do { $club } ({ $count } juniores)
satellites-column-parent-player = Jogador (Matriz / Emprestado)
satellites-column-position = Pos
satellites-column-age = Id
satellites-column-strength = Força
satellites-column-pitch = Campo
satellites-column-situation = Situação
satellites-column-value = Valor
satellites-column-satellite-pro = Profissional do Satélite
satellites-column-cost-parent = Custo p/ Matriz
satellites-column-junior = Júnior da Base do Satélite
satellites-column-current-strength = Força Atual
satellites-column-potential = Potencial (Olheiro)
satellites-column-rating = Avaliação
satellites-column-cost = Custo
satellites-button-lend = Emprestar ao Satélite
satellites-button-recall = Chamar de Volta
satellites-button-send = Ceder em Definitivo (R$ 0)
satellites-button-sign = Contratar p/ Matriz
satellites-button-pull-academy = Puxar p/ Minha Base
satellites-button-pull-pro = Promover p/ Meu Profissional
satellites-situation-loaned-in = Emprestado (de fora)
satellites-situation-young = Jovem (Sub-23)
satellites-situation-squad = No elenco
satellites-minutes = { $role } ({ $played }/{ $total }j, { $percent }%){ $warning }
satellites-minutes-warning = { " " }⚠
satellites-cost-your-loanee = Seu emprestado
satellites-cost-loanee = Emprestado
satellites-cost-group = R$ 0 (Grupo)
satellites-cost-discount = { $amount } (-25%)
satellites-title-warning = Aviso
satellites-title-buy-failed = Não foi possível comprar
satellites-title-partnership-failed = Não foi possível firmar parceria
satellites-amount-option = R$ { $amount }
satellites-inject-title = Injetar Caixa no Satélite
satellites-inject-text = Quanto deseja transferir do caixa do { $parent } para o { $club }?
    (100% do valor entra no caixa do satélite)
satellites-withdraw-nothing = O satélite não possui saldo excedente acima da reserva mínima de R$ 5,0 mi.
satellites-withdraw-all = Tudo disponível ({ $amount })
satellites-withdraw-title = Retirar Lucro do Satélite
satellites-withdraw-text = Quanto deseja retirar do caixa do { $club }?
    (Há retenção de 15% de impostos/taxas de repatriação; 85% entra no caixa da matriz)
satellites-buy-title = Comprar Clube Satélite
satellites-buy-text = Deseja adquirir 100% do controle do { $club } por { $price }?

    • Movimentação de jogadores e juniores a custo zero (R$ 0)
    • Controle de caixa e política operacional
    • Fim da cobrança de anuidade de parceria
satellites-end-title = Romper Parceria
satellites-end-text = Deseja rescindir a parceria com o { $club }?
    Multa rescisória (1 anuidade): { $fine }
satellites-sell-title = Vender Clube do Grupo
satellites-sell-text = Deseja vender o controle do { $club } por { $price } (90% da avaliação atual)?
    O valor entrará imediatamente no caixa do { $parent }.
satellites-pick-parent-player = Selecione um jogador do seu elenco na tabela à esquerda.
satellites-pick-loanee = Selecione um jogador emprestado ao satélite para chamá-lo de volta.
satellites-pick-pro = Selecione um profissional do satélite na tabela superior direita.
satellites-pick-junior = Selecione um júnior da base do satélite na tabela inferior direita.
satellites-pick-club-partnership = Selecione um clube na tabela para firmar parceria.
satellites-pick-club-buy = Selecione um clube na tabela para comprar.
satellites-role-title = Papel do Atleta no Empréstimo
satellites-role-text = Defina o papel esportivo acordado para { $player } no { $club }:
    O satélite priorizará escalá-lo como titular nas partidas em que ele estiver apto.
satellite-role-starter-option = Titular Absoluto (prioridade máxima, meta ~80% dos jogos)
satellite-role-regular-option = Titular Regular (rotação de time titular, meta ~60% dos jogos)
satellite-role-reserve-option = Reserva de Desenvolvimento (sem meta rígida)
satellites-the-satellite = satélite
satellites-recall-title = Recall por Descumprimento de Minutagem
satellites-recall-text = O jovem { $player } está com minutagem abaixo da meta no { $club }.
    Deseja acionar o recall imediato e reintegrá-lo ao elenco da matriz?
satellites-send-title = Ceder em Definitivo
satellites-send-text = Deseja transferir { $player } em definitivo para o { $club } a custo zero (R$ 0)?
satellites-sign-title = Contratar do Satélite
satellites-sign-text = Trazer { $player } (Força { $strength }) do { $club } para o { $parent } por { $price }?
satellites-sign-free = R$ 0 (transferência interna do grupo)
satellites-sign-discount = { $amount } (25% de desconto da parceria)
satellites-filter-country = País:
satellites-filter-size = Porte:
satellites-filter-filter = Filtro:
satellites-filter-order = Ordenar:
satellites-country-all = Todos os países
satellites-country-mine = Meu país ({ $country })
satellites-rep-any = Qualquer reputação
satellites-rep-0-1 = Reputação 0–1 (Regional)
satellites-rep-2 = Reputação 2 (Médio)
satellites-rep-3 = Reputação 3 (Tradicional)
satellites-rep-4 = Reputação 4 (Forte)
satellites-eligible = Elegíveis para vínculo
satellites-all-clubs = Todos os clubes do mundo
satellites-order-reputation = Maior reputação
satellites-order-price = Menor preço de compra
satellites-order-fee = Menor anuidade de parceria
satellites-order-academy = Maior academia de base
satellites-column-club = Clube
satellites-column-league = Liga / Divisão
satellites-column-reputation = Rep
satellites-column-squad-academy = Elenco / Base
satellites-column-cash = Caixa Est.
satellites-column-fee = Anuidade (Parceria)
satellites-column-price = Preço de Compra
satellites-column-status = Situação
satellites-squad-sizes = { $pros } pro / { $juniors } base
satellites-per-year = { $amount }/ano
satellites-situation-group = Grupo { $club }
satellites-situation-partner = Parceiro { $club }
satellites-situation-parent = Matriz de grupo
satellites-situation-billionaire = Dono bilionário (só parceria)
satellites-situation-same-division = Mesma divisão atual
satellites-situation-available = Disponível
satellites-search-tip = Parceria (3 anos): empréstimos ilimitados, 25% desconto em compras, 20% mais-valia e olheiro na base.  |  Compra (100%): atletas a R$ 0, controle de caixa e política.
satellites-button-partnership = Firmar Parceria (3 anos)
satellites-button-buy-new = Comprar Clube (100%)
satellites-partnership-title = Firmar Parceria
satellites-partnership-text = Firmar Parceria por Contrato (3 temporadas) com o { $club }?

    • Anuidade: { $fee } (1ª anuidade paga na assinatura)
    • Empréstimos garantidos sem limite e retorno imediato
    • 25% de desconto na compra de atletas do profissional e da base
    • 20% de mais-valia nas vendas do satélite para outros clubes
    • Relatório completo de olheiro da academia de base do satélite
satellites-buy-new-title = Comprar Clube (Dono de Bastidor)
satellites-buy-new-text = Adquirir 100% do controle do { $club } por { $price }?

    • Transferências definitivas e empréstimos a custo zero (R$ 0)
    • Captação direta de juniores da base a custo zero (R$ 0)
    • Controle da política operacional (Formar jovens, Buscar acesso ou Gerar lucro)
    • Injeção de caixa e retirada de lucros (-15% imposto)
    • Evolução +25% mais rápida para seus jovens emprestados ao satélite
satellites-networks-title = Redes Multiclubes e Parcerias Ativas no Mundo
satellites-history-title = Histórico Recente de Operações das Redes
satellites-column-parent = Clube Matriz
satellites-column-satellite = Clube Satélite
satellites-column-satellite-league = Liga do Satélite
satellites-column-link = Tipo de Vínculo
satellites-column-policy = Política / Contrato
satellites-column-since = Desde
satellites-column-year = Ano
satellites-column-operation = Operação
satellites-column-parent-short = Matriz
satellites-column-satellite-short = Satélite
satellites-column-details = Detalhes da Movimentação
satellites-you = { $club } (Você)
satellites-kind-owned = Clube Comprado (100%)
satellites-kind-partnership = Parceria por Contrato
satellites-fee-until = { $fee }/ano até { $year }
satellites-network-suspended = Suspenso (mesma div.)
satellites-network-active = Ativo
satellites-detail-partnership = Contrato de 3 anos ({ $fee }/ano)
satellites-detail-renewal = Parceria renovada até { $year } ({ $fee }/ano)
satellites-detail-fine = Multa paga: { $fine }
satellites-detail-contract-end = Fim de contrato
satellites-detail-bought = Comprado por { $price }
satellites-detail-sold = Vendido por { $price }
satellites-detail-injection = +{ $amount } transferidos ao satélite
satellites-detail-withdrawal = Retirado { $amount } (líquido { $net } após 15% taxas)
satellites-detail-assets = Modernizou { $pillar } (Nível { $level })
satellites-detail-loan = { $player } ({ $age }a, F{ $strength }) emprestado como { $role }
satellites-detail-return = { $player } chamado de volta à matriz
satellites-detail-free-group = R$ 0 (grupo)
satellites-detail-free = R$ 0
satellites-detail-discount = { $amount } (-25%)
satellites-detail-to-parent = { $player } (F{ $strength }) contratado por { $price }
satellites-detail-definitive = { $player } (F{ $strength }) cedido em definitivo ao satélite (R$ 0)
satellites-detail-to-pro = o profissional
satellites-detail-to-academy = a base
satellites-detail-youth = Júnior { $player } ({ $age }a) levado para { $to } por { $price }
satellites-detail-athlete = atleta
satellites-detail-abroad = exterior
satellites-detail-sell-on = +{ $amount } pela venda de { $player } ao { $buyer }
satellites-detail-ai-bought = Comprado por { $price } ({ $policy })
satellites-detail-ai-partnership = Parceria firmada ({ $fee }/ano)
satellites-detail-ai-loan-age = { $player } ({ $age }a) emprestado ao satélite
satellites-detail-ai-loan = { $player } emprestado ao satélite
satellites-detail-ai-promotion = { $player } (F{ $strength }) puxado pela matriz por { $price }
satellites-detail-ai-cession = { $player } cedido ao satélite (R$ 0)
satellites-assets-header = INSTALAÇÕES E PATRIMÔNIO DO SATÉLITE
satellites-assets-subtitle = { $club }  ·  Caixa da Matriz ({ $parent }): { $cash }
satellites-assets-intro = Financie a modernização da infraestrutura do satélite diretamente com os recursos da matriz.
satellites-assets-pillar = { $pillar }  ·  Nível { $level } / 5 ({ $name })
satellites-assets-next = Próximo nível: { $name } ({ $effect })
satellites-assets-max = Nível Máximo (5)
satellites-assets-upgrade = Evoluir p/ Nível { $level } ({ $cost })
satellites-assets-confirm-title = Confirmar Investimento no Satélite
satellites-assets-confirm = Deseja pagar { $cost } do caixa do { $parent }
    para elevar { $pillar } do { $club } para o Nível { $level } ({ $name })?
satellites-block-invalid = Clube inválido.
satellites-block-itself = Você não pode firmar vínculo com o próprio clube.
satellites-block-human = Clubes controlados por humanos ou seleções não podem ser satélites.
satellites-block-parent-is-satellite = Um clube que já é satélite não pode ser matriz de outro clube.
satellites-block-target-is-parent = O { $club } já é matriz de uma rede de clubes e não aceita virar satélite.
satellites-block-another-club = outro clube
satellites-block-satellite-of = O { $club } já é satélite do { $parent }.
satellites-block-already-linked = Você já possui vínculo ativo com o { $club }.
satellites-block-already-owned = O { $club } já pertence ao seu grupo.
satellites-block-reputation = O clube satélite precisa ter reputação menor que a da matriz ({ $target } vs { $parent }).
satellites-block-giant = Gigantes mundiais (reputação 5+) não estão à venda para grupos multiclubes.
satellites-block-investor = O { $club } pertence a um investidor bilionário e não está à venda.
satellites-error-suspended = Vínculo suspenso nesta temporada: matriz e satélite estão na mesma divisão nacional.
satellites-error-invalid-amount = Valor inválido.
satellites-error-cash-partnership = Caixa insuficiente para pagar a 1ª anuidade da parceria ({ $amount }).
satellites-error-cash-fine = Caixa insuficiente para pagar a multa rescisória de 1 anuidade ({ $amount }).
satellites-error-cash-buy = Caixa insuficiente para comprar o { $club } ({ $amount }).
satellites-error-cash-inject = A matriz não possui { $amount } em caixa para injetar.
satellites-error-cash-sign = Caixa insuficiente na matriz para contratar { $player } ({ $amount }).
satellites-error-cash-youth = Caixa insuficiente na matriz para trazer o júnior { $player } ({ $amount }).
satellites-error-working-capital = O satélite precisa manter ao menos { $amount } de capital de giro em caixa.
satellites-error-parent-cash = Caixa da matriz insuficiente: a obra custa { $cost } e a matriz tem { $cash }.
satellites-error-max-level = { $pillar } do satélite já está no nível máximo (Nível 5).
satellites-error-academy-full = A academia de juniores da matriz atingiu a capacidade máxima ({ $capacity } atletas).
satellites-error-no-active-partnership = Este clube não possui parceria por contrato ativa.
satellites-error-no-partnership = Este clube não possui parceria por contrato.
satellites-error-parent-missing = Clube matriz não encontrado.
satellites-error-not-your-satellite-selected = O clube selecionado não é seu satélite.
satellites-error-not-your-satellite = O clube não é seu satélite.
satellites-error-not-in-group = O clube não pertence ao seu grupo.
satellites-error-owned-inject = Apenas clubes comprados permitem injeção direta de caixa.
satellites-error-owned-withdraw = Apenas clubes comprados permitem retirada de lucro.
satellites-error-owned-invest = Apenas clubes comprados pelo grupo podem receber investimentos de patrimônio da matriz.
satellites-error-owned-send = Transferência definitiva gratuita para o satélite exige que o clube seja comprado pelo grupo.
satellites-error-not-parent-squad = O jogador não pertence ao elenco principal da matriz.
satellites-error-not-parent-player = O jogador não pertence à matriz.
satellites-error-not-satellite-junior = O júnior não pertence à base do satélite.
satellites-error-not-satellite-player = O jogador não pertence ao satélite.
satellites-error-ending-career = Este jogador já está encerrando a carreira.
satellites-error-on-loan-recall = Este jogador está emprestado. Use 'Chamar de volta' se ele pertencer à matriz.
satellites-error-on-loan-definitive = Jogador emprestado não pode ser transferido em definitivo.
satellites-error-on-loan = Este jogador já está emprestado.
satellites-error-not-on-loan = Este jogador não está emprestado.
satellites-error-parent-too-small = O elenco da matriz não pode ficar com menos de 16 jogadores.
satellites-error-satellite-full-named = O elenco do { $club } está lotado (máximo de 34 jogadores).
satellites-error-satellite-full = O elenco do satélite está lotado (máximo de 34 jogadores).
satellites-error-no-room = Não há vagas no elenco da matriz (limite de 35 jogadores).
satellites-error-parent-pro-full = O elenco profissional da matriz está lotado (máximo de 35 jogadores).
satellites-error-parent-full = O elenco da matriz está lotado (máximo de 35 jogadores).
satellites-error-satellite-too-small = O satélite não pode ficar com menos de 16 jogadores no elenco.

# Base.relatorio: the scouts' verdict on a junior
youth-report-star = Craque em potencial
youth-report-promise = Promessa
youth-report-squad = Jogador de elenco
youth-report-expendable = Dispensável
youth-report-range = { $low }–{ $high }
