export const GITHUB_USER_AGENT = "EdgeRun-Dash/0.1 (+https://dash.edgerun.tech)"

export function githubHeaders(token?: string): HeadersInit {
  return {
    Accept: "application/vnd.github+json",
    ...(token ? { Authorization: `Bearer ${token}` } : {}),
    "User-Agent": GITHUB_USER_AGENT,
    "X-GitHub-Api-Version": "2022-11-28",
  }
}
