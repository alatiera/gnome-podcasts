## Contributing to GNOME Podcasts

Thank you for looking in this file!

When contributing to the development of GNOME Podcasts, please first discuss the change you wish to make via issue, email, or any other method with the maintainers before making a change.

If you have any questions regarding the use or development of GNOME Podcasts,
want to discuss design or simply hang out, please join us in [#hammond on irc.gnome.org.](irc://irc.gnome.org/#hammond)

Please note we have a [code of conduct](https://wiki.gnome.org/Foundation/CodeOfConduc), please follow it in all your interactions with the project.

## Source repository

GNOME Podcasts's main source repository is at gitlab.gnome.org.  You can view
the web interface [here](https://gitlab.gnome.org/World/gnome-podcasts)

Development happens in the master branch.

Note that we don't do bug tracking in the Github mirror.

If you need to publish a branch, feel free to do it at any
publically-accessible Git hosting service, although gitlab.gnome.org
makes things easier for the maintainers.

## Style

We use [rustfmt](https://github.com/rust-lang-nursery/rustfmt) for code formatting and we enforce it on the gitlab-CI server.

Quick setup
```
cargo install rustfmt-nightly
cargo fmt --all
 ```

It is recommended to add a pre-commit hook to run cargo test and `cargo fmt`.
Don't forget to `git add` again after `cargo fmt`.
```
#!/bin/sh
cargo test -- --test-threads=1 && cargo fmt --all -- --write-mode=diff
```

## Running the test suite

Running the tests requires an internet connection and it it will download some files from the [Internet Archive](archive.org)

The test suite sets a temporary sqlite database in the `/tmp` folder.
Due to that it's not possible to run them in parrallel.

In order to run the test suite use the following: `cargo test -- --test-threads=1`

# Issues, issues and more issues!

There are many ways you can contribute to GNOME Podcasts, and all of them involve creating issues
in [GNOME Podcasts issue tracker](https://gitlab.gnome.org/World/gnome-podcasts/issues). This is the entry point for your contribution.

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

1. Open GNOME Podcasts
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
2. Ensure the test suit passes. Run `cargo test -- --test-threads=1`.
3. Ensure your code is properly formated. Run `cargo fmt --all`.
4. If you're adding new API, it must be properly documented.
5. The commit message is formatted as follows:
   ```
   component: <summary>

   A paragraph explaining the problem and its context.

   Another one explaining how you solved that.

   <link to the bug ticket>
   ```
6. You may merge the pull request in once you have the sign-off of the maintainers, or if you
   do not have permission to do that, you may request the second reviewer to merge it for you.

## Code of Conduct
We follow the Gnome [Code of Conduct.](https://wiki.gnome.org/Foundation/CodeOfConduct)
