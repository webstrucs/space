# Diretrizes de Codificação Python para Space Core

Este documento estabelece as diretrizes e restrições para o desenvolvimento da camada de alto nível em Python do projeto **Space Core**. O foco principal é garantir um ambiente de produção leve, seguro e fácil de auditar, além de promover um design de software robusto e reativo.

---

## 1. Versão do Python

A camada de alto nível do Space Core é desenvolvida e otimizada para o **Python 3.13.3**.

- É **mandatório** utilizar um ambiente virtual (`venv`) para isolar as dependências do projeto e garantir reprodutibilidade.

---

## 2. Restrição de Módulos Nativos

Para o ambiente de produção, há uma **restrição estrita** de utilizar **somente módulos nativos** da biblioteca padrão do Python.

### Implicações:

- **Segurança**: Reduz a superfície de ataque ao minimizar a inclusão de código de terceiros, que poderia introduzir vulnerabilidades desconhecidas.
- **Estabilidade e Confiabilidade**: Dependências externas podem ter seus próprios bugs, problemas de compatibilidade ou serem descontinuadas. Módulos nativos são mantidos pela comunidade Python e geralmente são mais estáveis e confiáveis a longo prazo.
- **Portabilidade**: Facilita a implantação em diferentes ambientes, pois não há necessidade de gerenciar um grande número de dependências externas ou compilá-las para plataformas específicas.
- **Tamanho da Imagem**: Reduz significativamente o tamanho final da imagem de implantação (ex: container Docker), o que é benéfico para o tempo de inicialização e o uso de recursos.
- **Auditoria e Manutenibilidade**: O código é mais fácil de auditar e manter, pois há menos camadas de abstração e menos "caixas pretas" de código de terceiros.

### Exceções (Apenas para Desenvolvimento/Ferramentas):

Ferramentas de desenvolvimento, testes, documentação ou scripts de build **podem utilizar módulos externos**, desde que **não sejam empacotados ou distribuídos com o código de produção**. Exemplos incluem:

- `pytest` para testes unitários.
- `black`, `isort` para formatação de código.
- `protobuf` (para gerar `messages_pb2.py` a partir do `.proto` do Rust). Este é um caso especial, pois a **saída é um módulo nativo Python**.

---

## 3. Design Pattern: Event-driven Architecture (EDA)

A arquitetura da camada Python será baseada em **Event-driven Architecture (EDA)**, utilizando `asyncio` para gerenciamento de I/O assíncrono.

### Princípios Chave:

- **Decoupling**: Componentes são fracamente acoplados, interagindo primariamente através da emissão e escuta de eventos, e não de chamadas diretas de função. Isso melhora a modularidade e a capacidade de manutenção.
- **Responsividade**: O uso de `asyncio` permite que o sistema permaneça responsivo mesmo sob alta carga, processando múltiplas operações de I/O concorrentemente sem bloquear o thread principal.
- **Escalabilidade**: A natureza assíncrona e desacoplada facilita a escalabilidade, permitindo que novos *handlers* de eventos ou *produtores* de eventos sejam adicionados sem impactar outros componentes.
- **Resiliência**: Falhas em um componente podem ser isoladas e tratadas sem derrubar todo o sistema, pois a comunicação é baseada em eventos.

### Implementação com `asyncio`:

- Todas as operações de I/O (comunicação IPC com o core Rust, interações com outros serviços, etc.) deverão ser implementadas usando `async/await` e `asyncio`.
- O uso de `asyncio.Queue` pode ser considerado para filas de eventos internas, e `asyncio.Event` ou `asyncio.Condition` para coordenação entre *coroutines*.

---

## 4. Tipos de Programação

A arquitetura do Space Core na camada Python abrange os seguintes paradigmas:

- **Programação Assíncrona**: Fundamental para *I/O-bound tasks*, gerenciada por `asyncio`.
- **Programação Distribuída**: Considerações para futuros serviços que podem se comunicar via rede, com ênfase em protocolos leves (`gRPC`, `TCP/UDP` customizado se necessário).
- **Programação Paralela**: Para *CPU-bound tasks*, caso seja necessário, o módulo `multiprocessing` da biblioteca padrão será o preferido. No entanto, o foco inicial é na assincronicidade para I/O.
- **Programação Híbrida**: A capacidade de combinar os paradigmas acima, utilizando o que for mais eficiente para cada tipo de tarefa.

---

## Conclusão

Ao aderir a estas diretrizes, garantiremos que a camada Python do **Space Core** seja eficiente, segura e fácil de expandir no futuro.
