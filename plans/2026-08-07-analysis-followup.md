# 실행 계획 — 2026-08-07 전체 분석 후속

출처: `docs/CODE_ANALYSIS_2026-08-07.md` §12. 기준선 `main` @ `4867ab9`, v0.145.0.

## 규약 (매 단계 공통)

1. `git switch -c <branch>` (main에서, 항상 `git pull` 후)
2. 수정 → `./scripts/verify.sh > /tmp/v.log 2>&1; echo $? > /tmp/v.exit` → **파일에서 코드 읽기**
   (`rm -f /tmp/v.exit` 선행). 문서/CI 전용 변경은 게이트 생략하고 보고서에 명시
3. 릴리스 범프는 `Cargo.toml` + `Cargo.lock` + `docs/CHANGELOG.md` 동시
4. 커밋 메시지는 **파일로** (`git commit -F`) — 인라인 `-m`은 로컬 verify-게이트 훅을 건드릴 수 있음
5. `gh pr create` → **CI green 확인** → `gh pr merge --squash` → `git switch main && git pull`
6. main 직접 푸시 금지 (훅이 차단)

## 단계

| # | 브랜치 | 커밋 | 버전 | 게이트 |
|---|---|---|---|---|
| 0 | — | (코드 변경 없음) | — | 워크플로 resume, 백그라운드 |
| 1 | `fix/gpu-particle-cursor-and-pipeline` | `fix(render): …` | 0.145.1 | verify + `cargo test --test render` (로컬 GPU) |
| 2 | `fix/docked-editor-post-chain` | `fix(render): …` | 0.145.2 | verify + docked headless capture |
| 3 | `fix/macos-gamepad-edge-accumulation` | `fix(input): …` | 0.145.3 | verify (신규 유닛 테스트 포함) |
| 4 | `fix/silent-data-loss` | `fix(save): …` | 0.145.4 | verify |
| 5 | `feat/fail-quiet-diagnostics` | `feat(ecs): …` | 0.146.0 | verify |
| 6 | `fix/reachable-panics` | `fix(engine): …` | 0.146.1 | verify |
| 7 | `feat/scene-reset-contract` | `feat(app): …` | 0.147.0 | verify |
| 8 | `feat/physics-collision-correctness` | `feat(physics): …` | 0.148.0 | verify |
| 9 | `fix/render-text-correctness` | `fix(render): …` | 0.148.1 | verify |
| 10 | `fix/audio-parity` | `fix(audio): …` | 0.148.2 | verify + 실기기 청취 |
| 11 | `feat/animation-correctness` | `feat(animation): …` | 0.149.0 | verify |
| 12 | `perf/per-frame-allocations` | `perf(engine): …` | 0.149.1 | verify |
| 13 | `docs/analysis-followup-drift` | `docs(*): …` | 없음 | 게이트 생략 (문서/CI) |

## 결정 사항 (착수 전 확정)

- **7번**: `AssetServer`/`ScriptRegistry`를 **persistent 등록**한다. CLAUDE.md 자신의 규칙
  ("config, device handles, **caches** → must persist")이 이쪽을 가리키고, 반대 선택(씬마다 해제)은
  기존 게임의 동작을 조용히 바꾼다. `PATTERNS.md`의 v0.139.1 감사 표를 함께 갱신.
- **공개 API 추가**(`despawn_with_body`, `AnimationPlayer::restart`)는 MINOR. 제거/이름변경은 없음.
- 예제 추가는 하지 않는다 — 전부 기존 기능의 버그 수정이라 새 기능이 아니다. 대신 각 수정에
  **회귀 테스트**를 붙인다(저장소의 "예제가 인수 테스트" 규약은 신규 기능에 적용).
