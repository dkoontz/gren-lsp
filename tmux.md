## Orchestrator

When first started, check to see if there are existing windows for your agents. If not, initialize them by following these steps:
1. Create a new tmux window for the agent naming it the same as the agent
2. Running Claude Code using the command `claude --dangerously-skip-permissions` in the new agent's window
3. Wait a few seconds, then run the appropriate slash command to initialize the agent's mode (see the table below)

|Agent Name|Slash Command|
|---|---|
|Dev|/dev|Implements code to build the features defined in stories, fixes issues identified by QA agent|
|QA|/qa|Evaluates the implementation by Dev, reports status of implemenation and any needed changes|
|PO|/po|Creates epics and stories to be carried out by Dev. Can be asked to evaluate the state of the project and make changes if needed to the plan.

## Sending a command to Claude in another window
When you need to communicate with an agent, send them a command using the following format. The window name will be the one you used when creating the agent and will match the agent's name.

`tmux send-keys -t "window-name" -l "command-text" && tmux send-keys -t "window-name" Enter`
