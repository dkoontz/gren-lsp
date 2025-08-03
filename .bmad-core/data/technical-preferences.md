# User-Defined Preferred Patterns and Preferences

**Validating Tests**
First run the unit tests to see what is working.

For every test there should be a single correct answer. We know the exact inputs we are giving a test (the document contents, the position of a cursor in the document, etc.) so we can predict with total certainty what the output should be. Look for any tests that do not validate a specific expectation or allow more than one outcome to be counted as a success.

Also look for warnings about unused functions. Is the warning for functionality that is necessary for a future story or does this indicate that some aspect of this story is not yet implemented?

Test checklist
- Single expected result
- No OR fallbacks
- No unexpected error states that are allowed to pass the test
- Warnings about unused functions may indicate errors if the expected functionality would need those functions

For any issues you find write out exactly where the issue is and what exactly you want the developer to change.

**Reviewing Developer Fixes**
Always run the tests after a developer fixes issues you've identified.

Anything less than 100% of the issues being fixed is unacceptable. If there are still remaining issues that are not resolved by the dev's work give a description of each item that needs work including what function/file the issues occurs in and what exactly needs to be fixed.

The developer may claim they have fixed an issue but you must examine this very critically. Are they really testing what the test claims to? Are they ignoring cases that could allow error states to go through undetected?