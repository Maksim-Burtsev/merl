#!/usr/bin/env python3
"""mkfix.py DEST: build DEST/services/{api,web,bot,docs} with bare origins in DEST/origin."""
import os, shutil, subprocess, sys

SRC = os.path.join(os.path.dirname(os.path.abspath(__file__)), "src")
DEST = sys.argv[1]
ENV = dict(os.environ, GIT_AUTHOR_NAME="acme", GIT_AUTHOR_EMAIL="dev@acme.test",
           GIT_COMMITTER_NAME="acme", GIT_COMMITTER_EMAIL="dev@acme.test",
           GIT_AUTHOR_DATE="2026-11-12T10:00:00", GIT_COMMITTER_DATE="2026-11-12T10:00:00")


for name, url, sha in (("tpl", "https://github.com/fastapi/full-stack-fastapi-template.git", "aabb901"),
                       ("bot", "https://github.com/andrew000/aiogram-template.git", "61ea147")):
    if not os.path.isdir(os.path.join(SRC, name)):
        subprocess.run(["git", "clone", "-q", url, os.path.join(SRC, name)], check=True)
        subprocess.run(["git", "-C", os.path.join(SRC, name), "checkout", "-q", sha], check=True)


def git(repo, *args):
    subprocess.run(["git", "-C", repo, *args], check=True, env=ENV,
                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


def edit(repo, rel, old, new):
    path = os.path.join(repo, rel)
    text = open(path, newline="").read()
    if "\r\n" in text:
        old, new = old.replace("\n", "\r\n"), new.replace("\n", "\r\n")
    assert old in text, (rel, old)
    open(path, "w", newline="").write(text.replace(old, new, 1))


def make(name, copy):
    repo = os.path.join(DEST, "services", name)
    origin = os.path.join(DEST, "origin", name + ".git")
    os.makedirs(repo)
    copy(repo)
    subprocess.run(["git", "init", "-q", "--bare", "-b", "main", origin], check=True)
    subprocess.run(["git", "init", "-q", "-b", "main", repo], check=True)
    git(repo, "add", "-A")
    git(repo, "commit", "-q", "-m", "import")
    git(repo, "remote", "add", "origin", origin)
    git(repo, "push", "-q", "-u", "origin", "main")
    git(repo, "remote", "set-head", "origin", "main")
    return repo


def branch(repo, name, message, edits):
    git(repo, "checkout", "-q", "-b", name, "main")
    for rel, old, new in edits:
        edit(repo, rel, old, new)
    git(repo, "commit", "-q", "-am", message)
    git(repo, "push", "-q", "origin", name)
    git(repo, "checkout", "-q", "main")
    git(repo, "branch", "-q", "-D", name)


tpl, bot = os.path.join(SRC, "tpl"), os.path.join(SRC, "bot", "app", "bot")
api = make("api", lambda d: [shutil.copytree(os.path.join(tpl, "backend", x), os.path.join(d, x))
                              for x in ("app", "scripts", "tests")])
web = make("web", lambda d: [shutil.copytree(os.path.join(tpl, "frontend", x), os.path.join(d, x))
                              for x in ("src", "public", "tests") if os.path.isdir(os.path.join(tpl, "frontend", x))])
docs = make("docs", lambda d: [shutil.copy(os.path.join(tpl, x), d)
                                for x in os.listdir(tpl) if x.endswith(".md")])
botr = make("bot", lambda d: shutil.copytree(bot, d, dirs_exist_ok=True))

branch(api, "ACME-56", "ACME-56: archive an item instead of deleting it", [
    ("app/models.py",
     "    description: str | None = Field(default=None, max_length=255)\n\n\n# Properties to receive on item creation",
     "    description: str | None = Field(default=None, max_length=255)\n    archived: bool = False\n\n\n# Properties to receive on item creation"),
    ("app/models.py",
     "    description: str | None = Field(default=None, max_length=255)\n\n\n# Database model",
     "    description: str | None = Field(default=None, max_length=255)\n    archived: bool | None = None\n\n\n# Database model"),
    ("app/api/routes/items.py",
     "    session: SessionDep, current_user: CurrentUser, skip: int = 0, limit: int = 100\n) -> Any:\n    \"\"\"\n    Retrieve items.\n    \"\"\"\n",
     "    session: SessionDep,\n    current_user: CurrentUser,\n    skip: int = 0,\n    limit: int = 100,\n    archived: bool = False,\n) -> Any:\n    \"\"\"\n    Retrieve items. Archived ones are left out unless asked for.\n    \"\"\"\n"),
    ("app/api/routes/items.py",
     "            .where(Item.owner_id == current_user.id)\n            .order_by(col(Item.created_at).desc())",
     "            .where(Item.owner_id == current_user.id)\n            .where(Item.archived == archived)\n            .order_by(col(Item.created_at).desc())"),
])
branch(web, "ACME-56", "ACME-56: an Archived badge in the items table", [
    ("src/components/Items/columns.tsx",
     "      <span className=\"font-medium\">{row.original.title}</span>\n",
     "      <span className=\"font-medium\">\n        {row.original.title}\n        {row.original.archived && (\n          <span className=\"ml-2 text-xs text-muted-foreground\">Archived</span>\n        )}\n      </span>\n"),
    ("src/components/Items/columns.tsx",
     "            !description && \"italic\",\n",
     "            !description && \"italic\",\n            row.original.archived && \"line-through\",\n"),
])
branch(docs, "ACME-56", "ACME-56: document archived items", [
    ("development.md",
     "## Mailpit\n",
     "## Archived items\n\nAn item is archived, not deleted: `PATCH /items/{id}` with `{\"archived\": true}`.\n`GET /items/` leaves archived items out; pass `?archived=true` to list them.\n\n## Mailpit\n"),
])
branch(web, "ACME-91", "ACME-91: initials of a one-word or empty name", [
    ("src/utils.ts",
     "  return name\n    .split(\" \")\n    .slice(0, 2)\n    .map((word) => word[0])\n",
     "  return name\n    .trim()\n    .split(/\\s+/)\n    .filter(Boolean)\n    .slice(0, 2)\n    .map((word) => word[0])\n"),
    ("src/utils.ts",
     "  return \"Something went wrong.\"\n",
     "  return err.message || \"Something went wrong.\"\n"),
])
branch(botr, "ACME-11", "ACME-11: log a deep link only when it is one", [
    ("handlers/cmds/start.py",
     "    deep_link = args[0]\n\n    logger.info(\"User %s started bot with deeplink: %s\", msg.from_user.id, deep_link)\n",
     "    if args:\n        logger.info(\"User %s started bot with deeplink: %s\", msg.from_user.id, args[0])\n"),
    ("handlers/cmds/start.py",
     "        disable_web_page_preview=True,\n    )\n",
     "        disable_web_page_preview=True,\n        disable_notification=True,\n    )\n"),
])
branch(api, "ACME-102", "ACME-102: cap the page size of the items list", [
    ("app/api/routes/items.py",
     "    Retrieve items.\n    \"\"\"\n",
     "    Retrieve items.\n    \"\"\"\n    limit = min(limit, 500)\n"),
])
branch(docs, "ACME-105", "ACME-105: say which Python the backend needs", [
    ("development.md",
     "Then, from the `backend` directory, install the dependencies and prepare the database:\n",
     "Then, from the `backend` directory (Python 3.12 or newer), install the dependencies and prepare the database:\n"),
])
print(DEST)
