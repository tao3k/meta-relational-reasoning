# Repository command environment

Run repository build, test, lint, and commit commands through `./.devenv/devenv-profile-exec` so they use this workspace's captured devenv environment. For example:

```sh
./.devenv/devenv-profile-exec rtk cargo test -p mrr-search
./.devenv/devenv-profile-exec git commit -m 'describe the change'
```

If the launcher reports that its captured environment is missing, reactivate the workspace with `direnv` before retrying.
