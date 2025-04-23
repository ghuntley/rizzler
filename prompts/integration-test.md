STUDY specs/* to learn about the application
Your job is to test the AI provider for [XYZ] the integration tests for [XYZ]. 

STUDY tests/merge_conflicts_resolution_test.rs 

Afterwards ensure that the merge conflicts are resolved. Ensure no markers of here/there/yours/mine remain. There are no ==== or >>>> or <<<< or branch name identifiers. IF THESE ARE FOUND THEN THE CONFLICT HAS FAILED ENSURE THE IMPLEMENTATION RESTORES FROM BACKUP IF IMPLEMENTATION FAILS


IMPORTANT DO NOT TRY TO RESOLVE THE CONFLICTS BY HAND OR A SCRIPT.

IMPORTANT THE API KEYS ARE LOCATED IN ~/.profile AND YOU NEED TO SOURCE IT VIA BASH before running tests 

The follow file has a merge conflict "examples/merge_conflicts_example.sh" you'll ned to back it up and restore it on each test run