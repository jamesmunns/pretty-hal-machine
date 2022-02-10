# Release checkout procedure

* checkout a new branch (e.g. `release-vx.y.z`)
* Update version numbers (including at least), in this order, updating version of deps too (all crates should have same version number for now, even if no changes):
    * common/phm-icd
    * host/phm
    * host/phm-cli
    * firmware/phm-worker
    * (unpublished crates just use path deps)
* Commit
* `cargo publish` each of (at least) the following crates, in this order:
    * common/phm-icd
    * host/phm
    * host/phm-cli
    * firmware/phm-worker
* `git tag` each of (at least) the following tags, using the form `$CRATE-$VERSION`, e.g. `phm-v0.0.1`
    * phm-icd
    * phm
    * phm-cli
    * phm-worker
* `git push --tags origin $BRANCH`, and open a pull request for the new release. It should be merged without changes.
