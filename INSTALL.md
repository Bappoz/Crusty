# INSTALL — preparando o ambiente

Guia para deixar o ambiente pronto para compilar e rodar o Crusty.

## Pré-requisitos

| Ferramenta | Versão mínima | Para quê |
|---|---|---|
| [Rust](https://rustup.rs/) (rustc + cargo) | 1.70+ | Compilar o próprio Crusty |
| `gcc` | qualquer versão recente | Montar (`as`) e linkar (`ld`) os executáveis ELF gerados pelo backend x86-64 |
| Linux x86-64 | — | O backend gera assembly x86-64 / System V ABI. Não há suporte a outras arquiteturas ou a Windows/macOS nativo |

Sem `gcc` no `PATH`, o compilador ainda funciona até a emissão de assembly (`--emit=asm`), mas os testes de smoke e2e (`tests/exe_smoke_test.rs`, `tests/codegen_smoke.rs`, `tests/double_codegen_test.rs`) são automaticamente pulados (skip), e `--emit=obj`/`--emit=exe` falham.

## 1. Instalar o Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
rustup update stable
```

Verifique:

```bash
rustc --version   # esperado: 1.70 ou mais recente
cargo --version
```

## 2. Instalar o gcc (toolchain de montagem/link)

**Debian/Ubuntu**
```bash
sudo apt update && sudo apt install -y gcc
```

**Arch/Manjaro**
```bash
sudo pacman -S gcc
```

**Fedora**
```bash
sudo dnf install gcc
```

Verifique:

```bash
gcc --version
```

## 3. Obter o código

```bash
git clone https://github.com/Bappoz/Crusty.git
cd Crusty
```

(Se você já está dentro do repositório, pule esta etapa.)

## 4. Compilar o projeto

```bash
cargo build --release
```

O binário fica em `target/release/crusty`. Para um build de desenvolvimento (mais rápido de compilar, binário mais lento):

```bash
cargo build
# binário em target/debug/crusty
```

## 5. Verificar a instalação

Rode o compilador sobre um exemplo incluso no repositório e execute o binário gerado:

```bash
cargo run --release -- src/examples/hello_world.c -o /tmp/hello
/tmp/hello
```

Saída esperada:

```
Hello, World!
```

Se isso funcionou, o ambiente está pronto. Para confirmar que toda a suíte de testes passa no seu ambiente:

```bash
cargo test --all
cargo clippy -- -D warnings
cargo fmt --check
```

Essas três checagens são exatamente as que o CI (`.github/workflows/`) roda em todo push/PR para `developer` e `master`.

## Problemas comuns

- **`error: linker 'cc' not found` ou falha ao montar/linkar** — `gcc` não está instalado ou não está no `PATH`. Repita o passo 2.
- **`cargo: command not found`** depois de instalar o Rust — rode `source "$HOME/.cargo/env"` ou abra um novo terminal.
- **Testes de smoke "pulando" silenciosamente** — esperado se `gcc` não estiver disponível; veja [TESTER.md](TESTER.md) para detalhes.
- **Programa de teste usa `float`** e falha com `error: code generation` — `float` ainda não tem codegen (limitação conhecida do backend, ver [README.md](README.md#limitações-conhecidas) e [issue #172](https://github.com/Bappoz/Crusty/issues/172)); `double` já é suportado.
