# VoxFlow

Ditado por voz gratuito para **macOS** e **Windows** — uma alternativa livre ao Wispr Flow.
Segure um atalho, fale, e o texto é transcrito (via [Groq](https://console.groq.com) + Whisper)
e colado automaticamente onde você estiver: e-mail, WhatsApp Web, editor de código, qualquer lugar.

Feito em [Tauri](https://tauri.app) (Rust + web). Janelinha flutuante leve, fica fora do seu caminho.

## Recursos

- 🎙️ **Ditado em qualquer app** — cola o texto no campo ativo.
- ⚡ **Rápido** — modelo Whisper *turbo* do Groq, com prévia ao vivo enquanto você fala.
- 🎯 **Preciso** — ao soltar o atalho, o áudio inteiro é transcrito de uma vez (sem cortes no meio das palavras).
- 🗣️ **Modo comando** — selecione um texto, fale "comando: deixa mais formal" e uma IA reescreve.
- 🌐 Português, inglês e espanhol.
- 🔒 **Privado** — sua API Key fica só na sua máquina; nada passa por servidor nosso.

## Instalar no macOS (Apple Silicon)

1. Baixe o `VoxFlow_*.dmg` mais recente em **[Releases](../../releases)**.
2. Abra o `.dmg` e arraste o **VoxFlow** para a pasta *Aplicativos*.
3. Como o app não é assinado pela Apple, na **primeira vez** clique com o botão direito no
   VoxFlow → **Abrir** → **Abrir**. (Se aparecer "está danificado", abra o Terminal e rode:
   `xattr -dr com.apple.quarantine /Applications/VoxFlow.app`)
4. O macOS vai pedir **duas permissões** — aceite as duas, senão não funciona:
   - **Microfone** (aparece sozinho na primeira gravação).
   - **Acessibilidade** — *Ajustes do Sistema → Privacidade e Segurança → Acessibilidade* →
     ative o VoxFlow. É o que permite ele colar o texto pra você.
5. Pegue uma **Groq API Key grátis** em <https://console.groq.com/keys>, clique na engrenagem ⚙
   da janelinha e cole.
6. Pronto: **segure F9**, fale, solte. O texto aparece.

> Dica: no Mac, as teclas F1–F12 às vezes controlam brilho/volume. Se o F9 não responder,
> use o **Fn+F9**, ou troque o atalho nas configurações (⚙) para uma combinação como
> ⌘+Shift+Espaço.

## Instalar no Windows

Baixe o `VoxFlow_*.msi` (ou `.exe`) em **[Releases](../../releases)**, instale, cole sua Groq API Key
na engrenagem ⚙ e segure **F9** para ditar.

## Como funciona o atalho

- **Segurar para falar** (padrão): segure o atalho enquanto fala, solte para inserir o texto.
- **Apertar p/ começar e parar**: aperte uma vez para gravar, aperte de novo para finalizar.
  (Útil para atalhos com combinação de teclas.) Troque nas configurações.

## Compilar do código

Pré-requisitos: [Rust](https://rustup.rs), [Node 20+](https://nodejs.org) e as dependências do Tauri
para o seu sistema (ver <https://tauri.app/start/prerequisites/>).

```bash
cargo install tauri-cli --version "^2"
cargo tauri icon app-icon.png   # gera os ícones a partir do app-icon.png
cargo tauri dev                 # rodar em desenvolvimento
cargo tauri build               # gerar o instalador
```

## Licença

MIT.
