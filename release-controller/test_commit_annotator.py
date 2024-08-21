from commit_annotator import target_determinator
from git_repo import GitRepo


def test_target_guestos_changed():
    ic_repo = GitRepo("https://github.com/dfinity/ic.git", main_branch="master")
    # not a guestos change
    assert target_determinator(object="00dc67f8d", ic_repo=ic_repo) == False
    # bumping dependencies
    assert target_determinator(object="2d0835bba", ic_repo=ic_repo) == True
    # .github change
    assert target_determinator(object="94fd38099", ic_repo=ic_repo) == False
    # replica change
    assert target_determinator(object="951e895c7", ic_repo=ic_repo) == True
    # modifies Cargo.lock but not in a meaningful way
    assert target_determinator(object="5a250cb34", ic_repo=ic_repo) == False
