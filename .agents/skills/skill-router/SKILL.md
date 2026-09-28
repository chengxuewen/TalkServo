---
name: skill-router
description: "Analyzes user intent and outputs recommended skill combinations. Auto-activates when user intent is ambiguous. Use when the user asks 'what skill should I use', 'how to approach this', or when intent is ambiguous."
---

# skill-router: skill routing analysis

> When user intent is ambiguous or multiple skills compete, analyzes and recommends the best skill combination.

## Trigger conditions

- User says '怎么做'/'用什么技能'/'帮我选择'  <!-- c1:allow-zh -->
- User intent is ambiguous and needs clarification before recommending
- Multiple skills may apply and comparison is needed

## Analysis flow

### Step 1: Intent classification

Read user message and classify into:

| Category | Keywords | Default recommendation |
|----------|----------|----------------------|
| **Implement** | '添加'/'实现'/'创建'/'修改' | `/think-before-act` + `/test-driven-development` |  <!-- c1:allow-zh -->
| **Fix** | '修复'/'bug'/'错误'/'不工作' | `/systematic-debugging` |  <!-- c1:allow-zh -->
| **Refactor** | '重构'/'优化'/'简化'/'清理' | `/think-before-act` + `/remove-ai-slops` |  <!-- c1:allow-zh -->
| **Design** | '设计'/'架构'/'方案'/'怎么做' | `/brainstorming` + `/openspec-propose` |  <!-- c1:allow-zh -->
| **Test** | '测试'/'E2E'/'覆盖率' | `/test-driven-development` + `/playwright` |  <!-- c1:allow-zh -->
| **Docs** | '文档'/'README'/'说明' | `/doc-audit` |  <!-- c1:allow-zh -->
| **Security** | '安全'/'权限'/'认证' | `/security-review` |  <!-- c1:allow-zh -->
| **Explore** | '调研'/'对比'/'有什么方案' | `/ecosystem-scan` + librarian agent |  <!-- c1:allow-zh -->
| **Ambiguous** | No clear keywords | Propose 2-3 possible directions, let user choose |

### Step 2: Context check

Before recommending, check current state:

1. **What in_progress tasks exist?** -- If tasks are in progress, only recommend skills relevant to the current task
2. **What skills did the user recently use?** -- Avoid re-recommending recently used skills
3. **What is the project's current phase?** -- New feature vs fix vs refactor, different phases recommend differently
4. **Tech stack match?** -- Ensure recommended skills apply to the current tech stack (Rust/TS/Docker/Web)

### Step 3: Output recommendation

**Single skill scenario** (clear intent):
```
Recommend: /systematic-debugging
Reason: 2 consecutive fix commits, suggesting systematic root-cause diagnosis is needed first
```

**Multi-skill scenario** (combination needed):
```
Recommended combination:
1. /think-before-act -- research approach first, avoid blind changes
2. /test-driven-development -- write tests before implementation
Reason: non-trivial multi-module change, needs test coverage
```

**Ambiguous scenario** (clarification needed):
```
Intent unclear, possible directions:
1. If implementing a new feature -> /think-before-act + /test-driven-development
2. If fixing a bug -> /systematic-debugging
3. If refactoring -> /think-before-act
What specifically do you want to do?
```

### Step 4: Execution suggestion

After recommending, if user confirms, invoke the skill directly:
```
Confirm: loading /think-before-act skill...
```

## Relationship with context-engineering

- **context-engineering**: passive trigger (recommends automatically when a scenario is detected, via the AGENTS.md directory table)
- **skill-router**: active invocation (user says '怎么做' or intent is ambiguous, performs deep analysis and recommendation)  <!-- c1:allow-zh -->

The two are complementary: context-engineering's routing table handles keyword matching, skill-router handles ambiguous intent and multi-skill combination recommendations.

## When NOT to recommend

| Scenario | Do NOT recommend | Reason |
|----------|-----------------|--------|
| User is urgently fixing | Any skill | Don't interrupt, finish the fix first |
| 1-line change | /think-before-act | Overkill for trivial edits |
| User explicitly names a skill | Other skills | Respect the user's choice |
| Same skill just recommended | Same skill | Avoid being pushy |
