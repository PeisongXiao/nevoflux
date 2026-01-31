# Findings & Decisions

## Requirements
- Remove native agent crates from `src/nevoflux/crates/`
- Configure nevoflux to use native agent from `/ai/project/nevoflux-agent`
- Maintain compatibility with existing extension (`agent@nevoflux.com`)

## Research Findings

### Current Setup Script (`scripts/setup-native-host.sh`)
- Builds cargo in `$PROJECT_ROOT/src/nevoflux/crates`
- Binary expected at: `src/nevoflux/crates/target/release/nevoflux-agent`
- Creates manifest at: `~/.mozilla/native-messaging-hosts/com.nevoflux.agent.json`

### New Project Setup Script (`/ai/project/nevoflux-agent/install/native-host/setup.sh`)
- More complete: supports Chrome, Firefox, Linux, macOS
- Binary path: `target/release/nevoflux` (configurable via argument)
- Same manifest name: `com.nevoflux.agent`
- Extension ID: `agent@nevoflux.com` (configurable)

### Extension Communication
- Extension connects via `browser.runtime.connectNative("com.nevoflux.agent")`
- 2-channel architecture: Chat + MCP
- No hardcoded binary paths in extension code

## Technical Decisions
| Decision | Rationale |
|----------|-----------|
| Update setup script to point to external project | Clean separation, single source of truth |
| Use `/ai/project/nevoflux-agent` as fixed path | Development convenience |

## Issues Encountered
| Issue | Resolution |
|-------|------------|

## Resources
- Old crates: `/ai/project/nevoflux/src/nevoflux/crates/`
- New agent: `/ai/project/nevoflux-agent/`
- Setup script: `/ai/project/nevoflux/scripts/setup-native-host.sh`
- Native manifest: `~/.mozilla/native-messaging-hosts/com.nevoflux.agent.json`

## Visual/Browser Findings
-

---
*Update this file after every 2 view/browser/search operations*
