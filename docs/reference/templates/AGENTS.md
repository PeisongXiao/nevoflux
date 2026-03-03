# NevoFlux Agents

> Protection level: L1 | Auto-learning — updated automatically from task execution
> Last updated: {timestamp}

## Task Execution Flow

1. **Parse** — Decompose user request into discrete sub-tasks
2. **Plan** — Determine tool and resource requirements for each sub-task
3. **Validate** — Check permissions and safety boundaries before execution
4. **Execute** — Run sub-tasks, capturing results and screenshots
5. **Verify** — Confirm outcomes match user intent
6. **Report** — Summarize results and surface any issues

## Failure Fallback Strategy

- **Level 1**: Retry with same strategy (transient errors)
- **Level 2**: Try alternative tool or selector strategy
- **Level 3**: Simplify the task (reduce scope or break into smaller steps)
- **Level 4**: Escalate to user with diagnosis and suggested next steps

## Multi-Task Orchestration

- Execute independent sub-tasks in parallel when safe
- Maintain dependency graph to sequence dependent tasks
- Share context between sub-tasks within the same session
- Cancel downstream tasks if an upstream dependency fails

## Learning System Integration

- Record successful task patterns for future reuse
- Update tool success rates after each invocation
- Refine selector strategies based on site-specific outcomes
- Feed error patterns back into fallback strategy ranking

## Session Collaboration

- Preserve context across turns within a session
- Support handoff between agent instances for long-running workflows
- Maintain a shared artifact store for intermediate results
- Allow user to bookmark and resume sessions
