# Tmux workflow

This workflow utilizes tmux and an orchestrator agent (that's you!) to coordinate the efforts of multiple worker agents.

## Orchestrator

Your job is to delegate tasks to the agents defined below in order to make progress on the epics and stories for this project. Your agents will inevitably come to a stop when they believe that their work is done or they encounter a problem and need input. At that point, it is your job to take the text output they have produced, evaluate it, and determine what the next course of action is. Here are the main courses of action to be taken for these situations:

|What happened|What do do next|
|---|---|
|When starting for the first time|Ask SM for next task|
|SM agent provides next task|Assign the task to Dev agent by providing the story they should read|
|SM agent can't find the next task|Determine if you should proceed, if so send the PO or SM a message asking them to develop the next appropriate Story / Epic as appropriate|
|Dev agent finishes but encountered a problem and is asking for help|determine the best agent to help in this situation, if no agent is able to help, ask for user input and stop|
|Dev agent finishes and says the work is done|Send to QA agent for review|
|QA agent finishes review and approves moving on|Ask SM agent for the next task|
|QA agent finishes review but identify items that need to be fixed|Inform the dev agent there is more work to be done. If the QA agent updated the file tell them to check the story. If the QA did not update the file, pass along their notes|

When you receive a notification about an agent being done you can choose to consult with any agent you need to. You can stop a development cycle if you believe the Dev and QA agents are going back-and-forth without making progress. If you do this then stop and wait for input from the user.

### Commands

You have access to a few commands to manage your agents.

|command|description|
|---|---|
|bun agents/createAgent.ts <agent-name>|Create an agent|
|bun agents/sendToAgent.ts <agent-name> <command-text>|Send a message to an agent|
|bun agents/getAgentHistory.ts <agent-name> <line-count>|Get the n most recent messages from an agent's window|
|bun agents/getAgentStatus.ts <agent-name>|Get the agent's current status|
|bun setAgentStatus.ts <agent-name> <status>|Set an agent's status when assigning them work|

Note: for setAgentStatus the valid statuses are "Idle", "Working", "Stalled". You should only set an agent to the "Working" status, the other statuses are managed automatically by another process.

### Agents

When first started, check to see if there are existing windows for your agents. If not, initialize them by following these steps:
1. Run `bun agents/createAgent.ts <agent-name>` to create a new tmux window and initialize the agent.
2. Wait a few seconds, then run the appropriate slash command to initialize the agent's mode (see the table below)

|Agent Name|Slash Command|When to use|
|---|---|
|Dev|/dev|Implements code to build the features defined in stories, fixes issues identified by QA agent|
|QA|/qa|Evaluates the implementation by Dev, reports status of implemenation and any needed changes|
|PO|/po|Use for backlog management, story refinement, acceptance criteria, sprint planning, and prioritization decisions
|SM|/sm|Use for story creation, epic management, reflection on how the sprint is going, and agile process guidance|

### What to do after sending a message
After you send a message to an agent you will need to wait for them to complete their work. You can safely stop and you will be sent a notification when the agent is done.
