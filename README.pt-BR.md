# CP2077 Mod Launcher

[English](README.md) | [Türkçe](README.tr.md) | [Русский](README.ru.md) | [Español](README.es.md) | [Português (BR)](README.pt-BR.md) | [Français](README.fr.md) | [Polski](README.pl.md) | [Deutsch](README.de.md)

Um gerenciador de mods não oficial, feito pela comunidade, para Cyberpunk 2077 no Windows. Versão 0.0.3 beta.

Ele não é feito por nem tem ligação com a CD PROJEKT RED, a Nexus Mods ou a equipe do NightCoop.

## O que ele faz

- Instala mods a partir de arquivos .zip, .rar e .7z. Solte o arquivo na janela ou clique em Instalar mod.
- Ativa um mod copiando os arquivos dele para a pasta do jogo e desativa apagando exatamente esses arquivos. Arquivos sobrescritos são salvos e devolvidos.
- Avisa antes de um mod sobrescrever um arquivo de outro mod.
- Encontra mods que você instalou à mão e deixa você importá-los.
- Junta várias versões de um mesmo mod em um único cartão, assim dá para ver qual é a atual.
- Verifica e instala versões do NightCoop pelo site oficial e confere cada download com SHA-256.
- Navega e busca na Nexus Mods com filtros, tags e rolagem infinita. Contas gratuitas podem instalar por links nxm ou pela pasta de downloads.
- Mostra o que cada mod da Nexus exige e compartilha sua lista de mods como perfil (um código ou um arquivo) para que outros instalem o que falta.
- Mostra os logs do jogo, dos mods e do launcher, e apaga quando você pedir.
- Fala inglês, turco, russo, espanhol, português do Brasil, francês, polonês e alemão.
- Tem temas do sistema, escuro e claro.

## Instalação

1. Baixe o zip em [Releases](https://github.com/This-null/cyberpunk-modloader/releases).
2. Descompacte e execute o instalador.
3. O Windows pode mostrar um aviso do SmartScreen porque o instalador não é assinado. Escolha Mais informações e depois Executar assim mesmo.

## Privacidade

Nada é enviado automaticamente. Os relatórios de erro só ficam salvos no seu computador. O launcher conversa com o site do NightCoop e com a Nexus Mods para listar e baixar mods. Ele também lê o número da versão mais recente no GitHub para avisar sobre atualizações. Dá para desligar em Configurações. As notas da versão e as descrições dos mods vão ao Google Tradutor só se você permitir em Configurações. Compartilhar um perfil online envia ao nosso servidor só a sua lista de mods (fontes e versões, sem arquivos) e um ID aleatório criado no seu computador, só quando você clica no botão, e o código expira em 12 horas. Sua chave de API da Nexus fica no Gerenciador de Credenciais do Windows do seu computador.

## Compilar a partir do código

Você precisa do Node 20 ou mais novo, pnpm e Rust estável.

```
pnpm install
pnpm tauri dev
pnpm tauri build
```

## Licença

MIT, veja [LICENSE](LICENSE). Arquivos RAR são descompactados com o código do UnRAR por meio do pacote unrar, que tem licença própria.

Use os mods por sua conta e risco e faça backup dos seus saves antes.
