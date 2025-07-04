# Guia de Contribuição e Metodologia

Este documento descreve as práticas de desenvolvimento e gerenciamento de projeto adotadas no Servidor Space.

## Fluxo de Trabalho Kanban

Utilizamos um quadro Kanban no GitHub Projects para gerenciar o fluxo de trabalho. As issues se movem pelas seguintes colunas:

### 1. 📋 To Do (A Fazer)
- **O que é:** O Backlog do projeto. Todas as issues que foram planejadas mas ainda não iniciadas.
- **Regra de Entrada:** Uma issue é adicionada aqui após ser criada e detalhada.
- **Regra de Saída:** Um desenvolvedor move a issue para "In Progress" quando começa a trabalhar nela.

### 2. 💻 In Progress (Em Progresso)
- **O que é:** Tarefas que estão sendo ativamente desenvolvidas.
- **Regra de Entrada:** A issue é puxada da coluna "To Do".
- **Regra de Saída:** Quando o desenvolvimento está completo e uma Pull Request (PR) é aberta para revisão, a issue é movida para "Review".

### 3. 🧐 Review (Em Revisão)
- **O que é:** O trabalho foi concluído e está aguardando revisão de código no Pull Request associado.
- **Regra de Entrada:** Uma PR é aberta e vinculada à issue.
- **Regra de Saída:** Se a PR for aprovada e mesclada (merged), a issue vai para "Done". Se a PR precisar de alterações, a issue pode voltar para "In Progress".

### 4. ✅ Done (Concluído)
- **O que é:** Tarefas concluídas, testadas e integradas à branch `dev`.
- **Regra de Entrada:** A PR associada foi mesclada com sucesso.
- **Regra de Saída:** A issue permanece aqui como parte do histórico de trabalho concluído.

## Etiquetas (Labels)

As etiquetas são usadas para categorizar as issues por área, prioridade ou tipo. Consulte a lista de labels diretamente na aba "Issues" do GitHub para ver as definições atuais.