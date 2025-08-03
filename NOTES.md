## Consider reverting intersectional capabililities
We filter our capabilities list to what is compatible with a client's capabilities. Some clients may not send any capabilities which would result in an empty capability set. This is an error on the part of the client but it might happen with some real-world clients. Since it is optional to filter the capablities set, it's safer to return the full capability list that we support.

## Create slash command for QA to check tests

The dev agent says they are done with Epic 2 Story 2. Analyze the requirements and look at the tests, specifically checking if the assertions made in the tests are actually testing what they claim to. Be sure to run the tests to see what test are passing. Append your analysis to the Epic 2 Story 2 document in a new section labelled `## QA Analysis`
