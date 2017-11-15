## Contributing

When contributing to the development of Hammond, please first discuss the change you wish to make via issue, email, or any other method with the maintainers before making a change.

Please note we have a code of conduct, please follow it in all your interactions with the project.

## Style

We use rustfmt for code formatting and we enforce it on the gitlab-CI server.

Quick setup
   ```
   cargo install rustfmt-nightly
   cargo fmt --all
   ```

It is recommended to add a pre-commit hook to run cargo test and cargo fmt
   ```
   #!/bin/sh
   cargo test --all && cargo fmt --all -- --write-mode=diff
   ```

# Issues, issues and more issues!

There are many ways you can contribute to Hammond, and all of them involve creating issues
in [Hammond issue tracker](https://gitlab.gnome.org/alatiera/Hammond/issues). This is the
entry point for your contribution.

To create an effective and high quality ticket, try to put the following information on your
ticket:

 1. A detailed description of the issue or feature request
     - For issues, please add the necessary steps to reproduce the issue.
     - For feature requests, add a detailed description of your proposal.
 2. A checklist of Development tasks
 3. A checklist of Design tasks
 4. A checklist of QA tasks

## Issue template
```
[Title of the issue or feature request]

Detailed description of the issue. Put as much information as you can, potentially
with images showing the issue or mockups of the proposed feature.

If it's an issue, add the steps to reproduce like this:

Steps to reproduce:

1. Open Hammond
2. Do an Action
3. ...

## Design Tasks

* [ ]  design tasks

## Development Tasks

* [ ]  development tasks

## QA Tasks

* [ ]  qa (quality assurance) tasks
```

## Pull Request Process

1. Ensure your code compiles. Run `make` before creating the pull request.
2. If you're adding new API, it must be properly documented.
3. The commit message is formatted as follows:
   ```
   component: <summary>

   A paragraph explaining the problem and its context.

   Another one explaining how you solved that.

   <link to the bug ticket>
   ```
4. You may merge the pull request in once you have the sign-off of the maintainers, or if you
   do not have permission to do that, you may request the second reviewer to merge it for you.

## Code of Conduct
We follow the Gnome [Code of Conduct.](https://wiki.gnome.org/Foundation/CodeOfConduct)
