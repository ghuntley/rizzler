STUDY specs/* to learn about the application

Run 
```
 source ~/.profile && rm -rf /tmp/rizzler*; make; RIZZLER_LOG_LEVEL=trace ./target/debug/rizzler resolve examples/merge_conflicts_example.sh; cp examples/merge_conflicts_example.sh /tmp/outcome
```

Afterwards ensure that the merge conflicts are resolved. Ensure no markers of here/there/yours/mine remain. There are no ==== or >>>> or <<<< or branch name identifiers

Take a backup copy of examples/merge_conflicts_example.sh and restore it on each attempt

You can view the results of each attempt by looking at /tmp/outcome

Let me know if it has passed.

IMPORTANT DO NOT TRY TO RESOLVE THE CONFLICTS BY HAND OR A SCRIPT.