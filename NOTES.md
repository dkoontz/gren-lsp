> @.bmad-core/agents/bmad-orchestrator.md  Help me plan the implementation of a LSP for the Gren programming langauge. I want to start by identifying all the messages that should be supported. I will be using Rust and the async-lsp crate. The documentation for the LSP spec is located at ./docs/lsp-spec/3.18. Perform research on the async-lsp crate and read through the LSP spec files.

I want to start with a document that details all the messages that will be supported, and contains this information for each one:
- The message
- The purpose of the message as defined by the spec
- At least one full JSON RPC example message sequence. For example `[{"sender": "client", "message":"document/open", "contents": "{the correct payload for a document open message}"},{"sender":"server","message":"whateverTheResponseShouldBe",contents":{the response message}"]`. This sequence should be sufficient to generate test cases where we send in one or more client message, receive back the responses from the server and then can assert the messages we receieved against the expected values. The client may be a curl process, an editor extension, etc. If there are multiple things to test with regards to a message, there should be multiple message sequences.

## Implementation notes
- The LSP server will rely solely on messages received from the client for its content.
- The LSP server will employ a local Gren compiler specified by the client via an environment variable to perform compilation
- The LSP server will maintain in-memory versions of files by applying edits. When a compilation needs to be triggered this in-memory file will be written to disk
- All tests will only communicate with the LSP server through its stdio interface and by sending/receiving LSP messages via JSON RPC.

Write out the test case file(s) in a file in the ./lsp directory.
Write out your notes on the PRD in ./docs/PRD.md
Write out your notes on the architecture in ./docs/architecture.md

## Create slash command for QA to check tests

The dev agent says they are done with Epic 1 Story 4. Analyze the requirements and look at the tests, specifically checking if the assertions made in the tests are actually testing what they claim to be. Be sure to run the tests to see what test are passing. Append your analysis to the Epic 1 Story 4 document in a new section labelled `## QA Analysis`
