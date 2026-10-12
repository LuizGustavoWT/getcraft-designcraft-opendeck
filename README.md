# DesignCraft Controls for OpenDeck

[Português do Brasil](#português-do-brasil) · [Contributing](CONTRIBUTING.md) · [DesignCraft](https://github.com/storytold/designcraft) · [OpenDeck](https://github.com/nekename/OpenDeck)

An independent Linux-only [OpenDeck](https://github.com/nekename/OpenDeck) plugin for practical [DesignCraft](https://github.com/storytold/designcraft) layout controls: live frame, layer, selection, alignment, save, and export actions plus rotary encoder control. It keeps an advanced raw-CLI action too, but the everyday actions operate the running layout app through its native control channel. It does not bundle DesignCraft: point it at the `designcraft-cli` installed on your machine.

Each action stores an executable path and a JSON array of arguments. The plugin launches the executable directly, without `sh -c` or another shell. That makes JSON parameters, spaces in paths, and untrusted filenames safe to pass as arguments.

## Features

- Focused **DesignCraft Layout Tool** action: create/open layouts, make text or graphic frames, add layers, duplicate/group/align selected items, save, and export tagged PDF/X-4.
- **DesignCraft Rotary** action for D200X and other encoders: canvas zoom, object nudging, and selected-object rotation with fit/save on encoder press.
- One configurable **Run DesignCraft CLI** action remains for advanced and future CLI features.
- Supports every DesignCraft CLI subcommand, option, and future addition: the action forwards the exact JSON argument array you configure.
- Loads the installed CLI's command or tool catalog in the Property Inspector and creates an app-specific, editable action template.
- Configurable executable path, working directory, timeout (1–300 seconds), label, and optional output summary on the key.
- Test a configuration in the Property Inspector before assigning it to a key.
- Linux x86_64 release ZIP created on every CI run; tagged versions are published as GitHub Releases.

## Requirements

- Linux x86_64
- OpenDeck with OpenAction-compatible plugin support
- DesignCraft and `designcraft-cli` installed and available in `PATH`, or its absolute path entered in the Property Inspector

Focused layout and rotary controls need DesignCraft running with its local control channel, for example `designcraft --control 7979`. The advanced CLI action remains useful for headless rendering and other scripted jobs.

## Install

1. Download `getcraft-designcraft-opendeck-linux-x86_64.zip` from the [latest release](../../releases/latest).
2. In OpenDeck, open **Plugins** → **Install from file** and select the ZIP. Do not extract or rename the `.sdPlugin` directory.
3. Drag **DesignCraft CLI → Run DesignCraft CLI** to a key.
4. Enter the CLI path if `designcraft-cli` is not in `PATH`, then enter a JSON argument array.
5. Use **Test command**. A green check means the CLI exited successfully; an alert means that its error summary is available in the Property Inspector and OpenDeck logs.

## Configuration and examples

The **Arguments** field must contain a JSON array of strings. It is not a shell command.

| Goal | Arguments |
| --- | --- |
| List the installed command catalog | `["commands"]` |

Use the exact argument syntax shown by `designcraft-cli --help` for project, file, export, and live-app control operations. The generic array accepts every documented subcommand and option.

The Property Inspector's **Load command catalog** button asks the installed CLI for the catalog supported by this application. Selecting an ID fills an app-specific, editable argument template; use **Test command** after adding the parameters and options required by your installed CLI version. The generic JSON array remains the canonical path to every CLI capability.

## Purpose-built Stream Deck actions

Start the desktop app with its documented local control channel, then add **DesignCraft Layout Tool** keys for your routine layout work:

```bash
designcraft --control 7979
```

Use one key for a new A4 document, a headline frame, a graphic frame, an Artwork layer, grouping, a common alignment, or a tagged PDF/X-4 export. The plugin sends DesignCraft's stable native command IDs directly to the running app; it never makes a shell command. PDF export always requires an explicit output path.

Assign **DesignCraft Rotary** to an encoder. Zoom preserves DesignCraft's native canvas behavior; nudge and rotation operate on the active selection in points or degrees. Press can fit the page/spread or save the active document.

## CI/CD and release

Pull requests and pushes to `main` run formatting checks, unit tests, and `scripts/package.sh`. The workflow uploads the Linux installer ZIP as an artifact. Pushing a semantic tag such as `v0.1.0` publishes the same ZIP as a GitHub Release.

To package locally:

```bash
rustup target add x86_64-unknown-linux-gnu
bash scripts/package.sh
```

The output is `dist/getcraft-designcraft-opendeck-linux-x86_64.zip`, containing exactly `wt.luizgustavo.getcraft.designcraft.sdPlugin/` at its root.

## Local test installation

On Linux, build the local installation package with:

```bash
bash scripts/install-local.sh
```

By default, the installer only builds the ZIP and does not change OpenDeck's active plugin directory. This prevents development plugins from disrupting an active setup. To deliberately test one plugin in the active OpenDeck installation, opt in explicitly:

```bash
OPENDECK_ALLOW_LIVE_INSTALL=1 bash scripts/install-local.sh
```

Install one development plugin at a time, then use OpenDeck's plugin reload control or restart it manually. The installer itself never restarts or reloads OpenDeck. To target a separate configuration, set `OPENDECK_PLUGINS_DIR` before running the command.

## Platform roadmap

| Platform | Status |
| --- | --- |
| Linux x86_64 | Supported now |
| Windows | Planned; not implemented or packaged yet |
| macOS | Planned; not implemented or packaged yet |

## Security and governance

The plugin never evaluates a shell command. It executes the chosen binary with the strings in the JSON array, and it limits a process to five minutes.

This is an open-contribution repository. Direct write/merge access is reserved for `LuizGustavoWT`; outside contributors use forks and pull requests. GitHub does not have a separate permission that only controls closing issues: users without repository write/triage access cannot close other users' issues, so retaining those permissions only for the maintainer satisfies the governance model. See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT. DesignCraft and OpenDeck are independent upstream projects with their own licenses and release policies.

---

# Português do Brasil

Plugin independente, somente para Linux, do [OpenDeck](https://github.com/nekename/OpenDeck) com controles práticos de diagramação do [DesignCraft](https://github.com/storytold/designcraft): ações ao vivo para frames, camadas, seleção, alinhamento, salvar e exportar, além de controle por encoder rotativo. A ação avançada de CLI livre continua disponível, mas as ações do dia a dia operam o aplicativo aberto pelo canal de controle nativo. O DesignCraft não é incluído no plugin: informe o `designcraft-cli` instalado na sua máquina.

Cada ação guarda o executável e uma lista JSON de argumentos. O plugin chama o executável diretamente, sem `sh -c` nem outro shell. Assim, JSON, caminhos com espaços e nomes de arquivos não confiáveis são passados com segurança.

## Recursos

- Ação focada **DesignCraft Layout Tool** para criar/abrir layouts, criar frames de texto ou gráfico, camadas, duplicar/agrupar/alinha itens selecionados, salvar e exportar PDF/X-4 marcado.
- **DesignCraft Rotary** para D200X e outros encoders: zoom do canvas, ajuste fino e rotação do objeto selecionado, com ajuste/salvamento no clique do encoder.
- Ação avançada **Run DesignCraft CLI** continua disponível para comandos livres e recursos futuros.
- Todos os subcomandos, opções e futuras adições da CLI: a ação repassa exatamente o vetor JSON configurado.
- Catálogo dinâmico de comandos ou ferramentas da CLI no Property Inspector, com modelo de ação editável e específico do aplicativo.
- Caminho do executável, diretório de trabalho, timeout (1–300 s), rótulo e resumo de saída configuráveis.
- Botão de teste no Property Inspector antes de associar a ação a uma tecla.
- ZIP Linux x86_64 gerado em toda execução da CI; tags são publicadas como Releases.

## Requisitos e instalação

Você precisa de Linux x86_64, OpenDeck compatível com OpenAction e DesignCraft com `designcraft-cli` no `PATH` (ou o caminho absoluto no Property Inspector).

Os controles focados de layout e rotary precisam do DesignCraft iniciado com o canal de controle local, por exemplo `designcraft --control 7979`. A ação avançada de CLI continua útil para renderização headless e outros trabalhos por script.

1. Baixe `getcraft-designcraft-opendeck-linux-x86_64.zip` na [release mais recente](../../releases/latest).
2. No OpenDeck, abra **Plugins** → **Install from file** e escolha o ZIP. Não extraia nem renomeie a pasta `.sdPlugin`.
3. Arraste **DesignCraft CLI → Run DesignCraft CLI** para uma tecla.
4. Configure o caminho da CLI, se necessário, e o vetor JSON de argumentos.
5. Use **Test command**. O check verde indica sucesso; o alerta mostra que houve erro, cujo resumo aparece no inspetor e nos logs do OpenDeck.

## Configuração

O campo **Arguments** precisa ser um vetor JSON de strings; ele não é um comando de shell.

| Objetivo | Arguments |
| --- | --- |
| Listar o catálogo de comandos instalado | `["commands"]` |

Use a sintaxe exata de `designcraft-cli --help` para operações de projeto, arquivo, exportação e controle do aplicativo em execução. O vetor genérico aceita todos os subcomandos e opções documentados.

**Load command catalog** consulta a CLI instalada pelo catálogo compatível com este aplicativo. Ao escolher um ID, o inspetor preenche um modelo de argumentos específico e editável; use **Test command** depois de adicionar os parâmetros e opções exigidos pela versão instalada da CLI. O vetor JSON genérico continua sendo o caminho para toda a CLI.

## Ações práticas para o Stream Deck

Inicie o aplicativo desktop com o canal de controle local e adicione teclas **DesignCraft Layout Tool** para o trabalho de diagramação:

```bash
designcraft --control 7979
```

Use uma tecla para novo A4, frame de título, frame gráfico, camada Artwork, agrupamento, alinhamento ou exportação PDF/X-4 marcada. O plugin envia os IDs de comando nativos do DesignCraft diretamente ao aplicativo aberto; não cria comandos de shell. A exportação exige sempre um caminho de saída explícito.

Atribua **DesignCraft Rotary** a um encoder. O zoom preserva o comportamento nativo do canvas; ajuste fino e rotação atuam na seleção ativa em pontos ou graus. Ao pressionar, ele pode ajustar página/spread ou salvar o documento ativo.

## CI/CD, roadmap e contribuição

PRs e pushes em `main` executam formatação, testes e `scripts/package.sh`. O ZIP vira artefato; uma tag como `v0.1.0` cria uma GitHub Release.

Para gerar o pacote local no Linux, execute `bash scripts/install-local.sh`. Por padrão, o instalador gera o ZIP e não altera o diretório de plugins ativo do OpenDeck, protegendo a configuração em uso. Para testar deliberadamente um único plugin na instalação ativa, confirme explicitamente:

```bash
OPENDECK_ALLOW_LIVE_INSTALL=1 bash scripts/install-local.sh
```

Instale um plugin de desenvolvimento por vez e então recarregue-o pelo OpenDeck ou reinicie manualmente. O instalador jamais reinicia ou recarrega o OpenDeck. Para usar outra configuração, defina `OPENDECK_PLUGINS_DIR` antes do comando.

| Plataforma | Situação |
| --- | --- |
| Linux x86_64 | Suportado agora |
| Windows | Planejado; ainda não implementado nem empacotado |
| macOS | Planejado; ainda não implementado nem empacotado |

Contribuições são bem-vindas via fork e pull request. Apenas `LuizGustavoWT` possui acesso direto para merge e para fechar issues de terceiros. O GitHub não oferece uma permissão separada somente para fechar issues; sem acesso de escrita/triagem, colaboradores públicos não conseguem fechar issues de outras pessoas. Veja [CONTRIBUTING.md](CONTRIBUTING.md).

## Licença

MIT. DesignCraft e OpenDeck são projetos upstream independentes, cada um com sua própria licença.
