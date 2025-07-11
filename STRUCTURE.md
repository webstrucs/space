# Estrutura de Diretórios do Projeto Space

Este documento descreve a organização dos diretórios na raiz do projeto, explicando o propósito de cada um.

-   `rs_core/`
    -   **Propósito:** Contém todo o código-fonte do núcleo de baixo nível em Rust. É um crate do Cargo independente, responsável pelo gerenciamento de rede, TLS, e a base de I/O de alta performance.

-   `core/`
    -   **Propósito:** Código central da camada de aplicação em Python. Inclui a lógica de negócio principal, serviços como gerenciamento de sessão, e utilitários como o construtor de respostas HTTP e o WAF.

-   `config/`
    -   **Propósito:** Armazena arquivos e scripts de configuração. Ideal para guardar configurações estáticas ou scripts de setup, como o de inicialização do banco de dados.

-   `routes/`
    -   **Propósito:** Contém as definições de roteamento da aplicação. O `router.py` neste diretório mapeia os padrões de URL para as funções de handler correspondentes.

-   `handlers/`
    -   **Propósito:** Contém as funções "controladoras" ou "handlers" que são executadas pelo roteador.

-   `interface/`
    -   **Propósito:** Módulos Python responsáveis pela comunicação direta com a camada Rust. Isso inclui a definição do protocolo IPC e a lógica de serialização/desserialização.

-   `static/`
    -   **Propósito:** Armazena arquivos estáticos que são parte do **próprio servidor**, não dos sites hospedados (ex: um painel de administração do servidor).

-   `log/`
    -   **Propósito:** Diretório padrão para a escrita de arquivos de log gerados pela aplicação em um ambiente de produção.

-   `works/`
    -   **Propósito:** Diretório principal que agrupa os diferentes tipos de "trabalhos" ou sites que o servidor pode hospedar. Estes diretórios serão, no futuro, gerenciados por uma CLI.
    -   `works/wse/`: (Web Sites Estáticos) Diretório raiz para servir websites estáticos.
    -   `works/wsd/`: (Web Sites Dinâmicos) Diretório raiz para as aplicações dinâmicas, contendo templates e scripts.
    -   `works/wfc/`: (Web Frameworks Customizados) Diretório para hospedar o código de frameworks web desenvolvidos sob medida.
    -   `works/wfp/`: (Web Frameworks Populares) Contém os adaptadores e configurações para rodar frameworks populares via WSGI/ASGI.