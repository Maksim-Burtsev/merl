import random
random.seed(409)
teams = ["falcon", "harbor", "lumen", "quartz"]
words = "fix login export pdf search cart refund coupon payment profile avatar upload email digest timeout retry cache invoice report filter sort page mobile layout tests migrate api rate limit webhook".split()
nums = random.sample(range(1000, 9999), 200)
out = {"dev-harbor/1234-fix-login", "dev-lumen/11234-export-pdf"}
for n in nums:
    if "1234" in str(n):
        continue
    out.add(f"dev-{random.choice(teams)}/{n}-{'-'.join(random.sample(words, random.randint(1, 4)))}")
alnum = "abcdefghijklmnopqrstuvwxyz0123456789"
for _ in range(40):
    out.add("claude/task-" + "".join(random.choice(alnum) for _ in range(6)))
out |= {"claude/task-2ars83", "agent/refund", "agent/fix-login-rate", "codex/add-coupons",
        "claude/implement-the-whole-review-queue-with-briefs-and-notes-for-every-branch-2ars84",
        "release/2.10.1", "hotfix/v1.2.3-rc.1", "chore/deps_bump", "Topic/CamelCase-Name",
        "fix--double--dash", "user@laptop/fix", "release@2026.10", "feature/café-menu",
        "fix/日本語-labels", "фича/поиск-заказов", "wip/🚀-launch", "feat/c#-support", "fix/a+b=c",
        "fix/comma,list", "fix/it's-done", "fix/dollar$HOME", "fix/amp&co", "fix/paren(1)",
        "fix/semi;colon", "fix/pipe|x", "fix/brace{x}", "fix/excl!", "fix/percent%20",
        "fix/angle<x>", "fix/back`tick", "fix/quote\"x", "deep/a/b/c/d/e/f"}
print("\n".join(sorted(out)))
