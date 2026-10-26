# The Patches screen: the data sources of a new game (docs/patches.md)

opening-patches = Patches
opening-patches-detail = Times, ligas e elencos do GitHub

patches-title = Patches
patches-subtitle = De onde vêm os times, elencos e ligas do Novo jogo
patches-sources = Fontes
patches-order-hint = A primeira é a base. Cada fonte seguinte troca ou acrescenta times, elencos e ligas sobre as anteriores. Carreiras em andamento não mudam.
patches-kind-base = Base
patches-kind-patch = Patch
patches-official = Oficial
patches-version = versão { $version }
patches-enabled = Usar no Novo jogo
patches-not-downloaded = Não baixado
patches-up-to-date = Atualizado
patches-update-available = Atualização disponível
patches-checking = Procurando atualização…
patches-folder-live = Pasta lida do disco a cada Novo jogo
patches-download = Baixar
patches-update = Atualizar
patches-retry = Tentar de novo
patches-remove = Remover
patches-move-up = Subir
patches-move-down = Descer
patches-megabytes = { $value } MB

patches-phase-starting = Conectando ao GitHub…
patches-phase-chunks = Baixando { $done } de { $total }
patches-phase-files = Baixando arquivo { $done } de { $total }
patches-phase-waiting = O GitHub limitou os downloads. Continua sozinho em { $seconds } s
patches-phase-saving = Salvando…

patches-error-network = Sem conexão com o GitHub ({ $reason })
patches-error-not-found = Repositório não encontrado, ou privado
patches-error-not-a-pack = Não é um pacote de dados: falta o manifest.toml
patches-error-unsupported-format = O formato { $format } é de uma versão mais nova do jogo
patches-error-corrupted = O pacote mudou durante o download. Tente de novo
patches-error-storage = Não deu para salvar: { $reason }
patches-error-rate-limited = O GitHub limitou os downloads. Tente mais tarde

patches-add = Adicionar fonte
patches-add-github = Repositório do GitHub
patches-add-github-hint = https://github.com/dono/repositorio
patches-add-button = Adicionar
patches-add-invalid = Isso não é um link de repositório do GitHub
patches-add-duplicate = Essa fonte já está na lista
patches-add-path = Pasta ou arquivo .zip no disco
patches-add-path-hint = Caminho da pasta do pacote ou do .zip
patches-add-folder-button = Adicionar pasta
patches-add-zip-button = Adicionar zip
patches-add-path-missing = Caminho não encontrado

patches-database = Banco do Novo jogo
patches-database-summary = { $clubs } clubes · { $players } jogadores · { $leagues } ligas · { $states } estaduais
patches-database-empty = Nenhuma fonte baixada. Baixe o banco oficial para começar um jogo.
patches-database-loading = Montando o banco…
patches-skipped = { $count ->
    [one] 1 fonte marcada ainda não foi baixada
   *[other] { $count } fontes marcadas ainda não foram baixadas
}
patches-issues = { $errors ->
    [one] 1 erro
   *[other] { $errors } erros
} e { $warnings ->
    [one] 1 aviso
   *[other] { $warnings } avisos
}
patches-show-issues = Ver problemas
patches-hide-issues = Esconder problemas
patches-issues-more = e mais { $count }
patches-help = Como montar um patch

new-game-no-data = Nenhum banco de times baixado
new-game-no-data-detail = Os times, elencos e ligas vêm da tela Patches. Baixe o banco oficial (ou outra fonte) para começar.
new-game-open-patches = Abrir Patches

# What the validator finds wrong in a pack (brasfoot_datapack::issues)
patches-field-level = nível
patches-field-capacity = capacidade
patches-field-age = idade
patches-issue-unreadable = não deu para ler o arquivo: { $reason }
patches-issue-syntax = { $reason }
patches-issue-missing-manifest = falta o manifest.toml na raiz
patches-issue-unsupported-format = formato { $found }, mais novo que o desta versão do jogo
patches-issue-patch-without-base = um patch precisa de uma base antes dele
patches-issue-removals-in-base = só um patch remove coisas
patches-issue-unknown-file = arquivo fora do formato, ignorado
patches-issue-bad-folder-name = a pasta "{ $name }" deve ter só letras minúsculas, números e hífen
patches-issue-missing-club-file = pasta de clube sem club.toml
patches-issue-missing-squad = clube sem squad.csv
patches-issue-unknown-club = muda um clube que nenhuma fonte anterior tem
patches-issue-country-folder-mismatch = clube de { $country } na pasta de { $folder }
patches-issue-file-name-mismatch = o arquivo deveria se chamar { $expected }
patches-issue-unknown-country = país "{ $code }" desconhecido (use o trigrama da FIFA)
patches-issue-unknown-state = estado "{ $code }" desconhecido
patches-issue-state-outside-brazil = só clubes do Brasil têm estado
patches-issue-missing-state = clube brasileiro sem estado, não joga estadual
patches-issue-empty-name = nome vazio
patches-issue-bad-color = a cor "{ $value }" não está no formato #rrggbb
patches-issue-out-of-range = { $field } { $value } fora de { $min } a { $max }; o jogo troca o valor
patches-issue-potential-off-scale = potencial { $value } fora da escala de 0 a 10; o jogo usa assim mesmo
patches-issue-unknown-position = posição "{ $code }" desconhecida
patches-issue-unknown-side = lado "{ $code }" desconhecido
patches-issue-unknown-trait = característica "{ $code }" desconhecida
patches-issue-unknown-rival = o rival { $id } não está no banco
patches-issue-missing-crest = clube sem crest.png
patches-issue-bad-division = divisão { $division } fora de 1 a 4
patches-issue-duplicate-division = divisão { $division } configurada duas vezes
patches-issue-unknown-state-format = formato de estadual "{ $code }" desconhecido
patches-issue-unknown-removal = remove { $id }, que nenhuma fonte anterior tem
