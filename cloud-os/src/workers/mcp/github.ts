/**
 * GitHub MCP Server
 * Exposes GitHub API capabilities as MCP tools
 */

import { MCPServerBase, setupWorkerServer } from './base';

const GITHUB_API_BASE = 'https://api.github.com';

class GitHubServer extends MCPServerBase {
  constructor() {
    super('github', '1.0.0');
  }

  setupHandlers(): void {
    // Register tools
    this.registerTool(
      {
        name: 'github_list_repos',
        description: 'List repositories for the authenticated user',
        inputSchema: {
          type: 'object',
          properties: {
            sort: {
              type: 'string',
              enum: ['created', 'updated', 'pushed', 'full_name'],
              default: 'updated',
            },
            limit: {
              type: 'number',
              default: 30,
            },
          },
        },
      },
      async (args) => {
        const token = await this.getToken();
        if (!token) {
          return {
            content: [{ type: 'text', text: 'Not authenticated. Please connect GitHub first.' }],
            isError: true,
          };
        }

        try {
          const response = await fetch(
            `${GITHUB_API_BASE}/user/repos?sort=${args.sort || 'updated'}&per_page=${args.limit || 30}`,
            {
              headers: {
                'Authorization': `Bearer ${token}`,
                'Accept': 'application/vnd.github.v3+json',
              },
            }
          );

          if (!response.ok) {
            throw new Error(`GitHub API error: ${response.statusText}`);
          }

          const repos = await response.json();
          
          return {
            content: [{
              type: 'text',
              text: JSON.stringify(repos.map((r: any) => ({
                name: r.name,
                full_name: r.full_name,
                description: r.description,
                url: r.html_url,
                stars: r.stargazers_count,
                language: r.language,
                updated: r.updated_at,
              })), null, 2),
            }],
          };
        } catch (error) {
          return {
            content: [{ type: 'text', text: `Error: ${error instanceof Error ? error.message : 'Unknown error'}` }],
            isError: true,
          };
        }
      }
    );

    this.registerTool(
      {
        name: 'github_get_repo',
        description: 'Get details of a specific repository',
        inputSchema: {
          type: 'object',
          properties: {
            owner: {
              type: 'string',
              description: 'Repository owner',
            },
            repo: {
              type: 'string',
              description: 'Repository name',
            },
          },
          required: ['owner', 'repo'],
        },
      },
      async (args) => {
        const token = await this.getToken();
        
        try {
          const response = await fetch(
            `${GITHUB_API_BASE}/repos/${args.owner}/${args.repo}`,
            {
              headers: token ? {
                'Authorization': `Bearer ${token}`,
                'Accept': 'application/vnd.github.v3+json',
              } : {
                'Accept': 'application/vnd.github.v3+json',
              },
            }
          );

          if (!response.ok) {
            throw new Error(`GitHub API error: ${response.statusText}`);
          }

          const repo = await response.json();
          
          return {
            content: [{
              type: 'text',
              text: JSON.stringify({
                name: repo.name,
                full_name: repo.full_name,
                description: repo.description,
                url: repo.html_url,
                stars: repo.stargazers_count,
                forks: repo.forks_count,
                language: repo.language,
                default_branch: repo.default_branch,
                updated: repo.updated_at,
              }, null, 2),
            }],
          };
        } catch (error) {
          return {
            content: [{ type: 'text', text: `Error: ${error instanceof Error ? error.message : 'Unknown error'}` }],
            isError: true,
          };
        }
      }
    );

    this.registerTool(
      {
        name: 'github_list_issues',
        description: 'List issues in a repository',
        inputSchema: {
          type: 'object',
          properties: {
            owner: {
              type: 'string',
              description: 'Repository owner',
            },
            repo: {
              type: 'string',
              description: 'Repository name',
            },
            state: {
              type: 'string',
              enum: ['open', 'closed', 'all'],
              default: 'open',
            },
            limit: {
              type: 'number',
              default: 30,
            },
          },
          required: ['owner', 'repo'],
        },
      },
      async (args) => {
        const token = await this.getToken();
        
        try {
          const response = await fetch(
            `${GITHUB_API_BASE}/repos/${args.owner}/${args.repo}/issues?state=${args.state || 'open'}&per_page=${args.limit || 30}`,
            {
              headers: token ? {
                'Authorization': `Bearer ${token}`,
                'Accept': 'application/vnd.github.v3+json',
              } : {
                'Accept': 'application/vnd.github.v3+json',
              },
            }
          );

          if (!response.ok) {
            throw new Error(`GitHub API error: ${response.statusText}`);
          }

          const issues = await response.json();
          
          return {
            content: [{
              type: 'text',
              text: JSON.stringify(issues.map((i: any) => ({
                number: i.number,
                title: i.title,
                state: i.state,
                url: i.html_url,
                user: i.user.login,
                created: i.created_at,
                labels: i.labels.map((l: any) => l.name),
              })), null, 2),
            }],
          };
        } catch (error) {
          return {
            content: [{ type: 'text', text: `Error: ${error instanceof Error ? error.message : 'Unknown error'}` }],
            isError: true,
          };
        }
      }
    );

    this.registerTool(
      {
        name: 'github_create_issue',
        description: 'Create a new issue in a repository',
        inputSchema: {
          type: 'object',
          properties: {
            owner: {
              type: 'string',
              description: 'Repository owner',
            },
            repo: {
              type: 'string',
              description: 'Repository name',
            },
            title: {
              type: 'string',
              description: 'Issue title',
            },
            body: {
              type: 'string',
              description: 'Issue body',
            },
          },
          required: ['owner', 'repo', 'title'],
        },
      },
      async (args) => {
        const token = await this.getToken();
        if (!token) {
          return {
            content: [{ type: 'text', text: 'Not authenticated. Please connect GitHub first.' }],
            isError: true,
          };
        }

        try {
          const response = await fetch(
            `${GITHUB_API_BASE}/repos/${args.owner}/${args.repo}/issues`,
            {
              method: 'POST',
              headers: {
                'Authorization': `Bearer ${token}`,
                'Accept': 'application/vnd.github.v3+json',
                'Content-Type': 'application/json',
              },
              body: JSON.stringify({
                title: args.title,
                body: args.body,
              }),
            }
          );

          if (!response.ok) {
            throw new Error(`GitHub API error: ${response.statusText}`);
          }

          const issue = await response.json();
          
          return {
            content: [{
              type: 'text',
              text: `Created issue #${issue.number}: ${issue.title}\n${issue.html_url}`,
            }],
          };
        } catch (error) {
          return {
            content: [{ type: 'text', text: `Error: ${error instanceof Error ? error.message : 'Unknown error'}` }],
            isError: true,
          };
        }
      }
    );

    this.registerTool(
      {
        name: 'github_search_code',
        description: 'Search code across GitHub',
        inputSchema: {
          type: 'object',
          properties: {
            query: {
              type: 'string',
              description: 'Search query',
            },
            language: {
              type: 'string',
              description: 'Filter by language',
            },
            limit: {
              type: 'number',
              default: 30,
            },
          },
          required: ['query'],
        },
      },
      async (args) => {
        const token = await this.getToken();

        let query = args.query;
        if (args.language) {
          query += ` language:${args.language}`;
        }

        try {
          const response = await fetch(
            `${GITHUB_API_BASE}/search/code?q=${encodeURIComponent(query)}&per_page=${args.limit || 30}`,
            {
              headers: token ? {
                'Authorization': `Bearer ${token}`,
                'Accept': 'application/vnd.github.v3+json',
              } : {
                'Accept': 'application/vnd.github.v3+json',
              },
            }
          );

          if (!response.ok) {
            throw new Error(`GitHub API error: ${response.statusText}`);
          }

          const result = await response.json();

          return {
            content: [{
              type: 'text',
              text: JSON.stringify({
                total: result.total_count,
                items: result.items.map((item: any) => ({
                  name: item.name,
                  path: item.path,
                  repository: item.repository.full_name,
                  url: item.html_url,
                })),
              }, null, 2),
            }],
          };
        } catch (error) {
          return {
            content: [{ type: 'text', text: `Error: ${error instanceof Error ? error.message : 'Unknown error'}` }],
            isError: true,
          };
        }
      }
    );

    // Register get_commits tool
    this.registerTool(
      {
        name: 'github_get_commits',
        description: 'Get commit history for a repository',
        inputSchema: {
          type: 'object',
          properties: {
            owner: {
              type: 'string',
              description: 'Repository owner',
            },
            repo: {
              type: 'string',
              description: 'Repository name',
            },
            branch: {
              type: 'string',
              description: 'Branch name (defaults to default branch)',
            },
            limit: {
              type: 'number',
              description: 'Maximum number of commits',
              default: 10,
            },
            sha: {
              type: 'string',
              description: 'SHA or branch to start from',
            },
          },
          required: ['owner', 'repo'],
        },
      },
      async (args) => {
        const token = await this.getToken();

        try {
          let url = `${GITHUB_API_BASE}/repos/${args.owner}/${args.repo}/commits?per_page=${args.limit || 10}`;
          if (args.branch) url += `&sha=${args.branch}`;
          if (args.sha) url += `&sha=${args.sha}`;

          const response = await fetch(url, {
            headers: token ? {
              'Authorization': `Bearer ${token}`,
              'Accept': 'application/vnd.github.v3+json',
            } : {
              'Accept': 'application/vnd.github.v3+json',
            },
          });

          if (!response.ok) {
            throw new Error(`GitHub API error: ${response.statusText}`);
          }

          const commits = await response.json();

          return {
            content: [{
              type: 'text',
              text: commits.map((c: any) => 
                `${c.sha.slice(0, 7)} - ${c.commit.message.split('\n')[0]} (${c.commit.author.name}, ${c.commit.author.date.split('T')[0]})`
              ).join('\n'),
            }],
            ui: {
              viewType: 'timeline',
              title: 'Commit History',
              description: `Last ${commits.length} commits from ${args.owner}/${args.repo}`,
              metadata: {
                source: 'github',
                owner: args.owner,
                repo: args.repo,
                branch: args.branch || 'default',
                itemCount: commits.length,
                timestamp: new Date().toISOString(),
              },
              events: commits.map((c: any) => ({
                id: c.sha,
                title: c.commit.message.split('\n')[0],
                description: c.commit.message,
                timestamp: c.commit.author.date,
                author: c.commit.author.name,
                avatar: c.author?.avatar_url,
                type: 'commit',
              })),
            },
          };
        } catch (error) {
          return {
            content: [{ type: 'text', text: `Error: ${error instanceof Error ? error.message : 'Unknown error'}` }],
            isError: true,
          };
        }
      }
    );

    // Register get_diff tool
    this.registerTool(
      {
        name: 'github_get_diff',
        description: 'Get code diff between branches or for a pull request',
        inputSchema: {
          type: 'object',
          properties: {
            owner: {
              type: 'string',
              description: 'Repository owner',
            },
            repo: {
              type: 'string',
              description: 'Repository name',
            },
            base: {
              type: 'string',
              description: 'Base branch (e.g., main)',
            },
            head: {
              type: 'string',
              description: 'Head branch (e.g., feature-branch)',
            },
            pr: {
              type: 'number',
              description: 'Pull request number (alternative to base/head)',
            },
          },
          required: ['owner', 'repo'],
        },
      },
      async (args) => {
        const token = await this.getToken();

        try {
          let url: string;
          if (args.pr) {
            url = `${GITHUB_API_BASE}/repos/${args.owner}/${args.repo}/pulls/${args.pr}`;
          } else if (args.base && args.head) {
            url = `${GITHUB_API_BASE}/repos/${args.owner}/${args.repo}/compare/${args.base}...${args.head}`;
          } else {
            return {
              content: [{ type: 'text', text: 'Error: Must provide either PR number or both base and head branches' }],
              isError: true,
            };
          }

          const response = await fetch(url, {
            headers: token ? {
              'Authorization': `Bearer ${token}`,
              'Accept': 'application/vnd.github.v3.diff',
            } : {
              'Accept': 'application/vnd.github.v3.diff',
            },
          });

          if (!response.ok) {
            throw new Error(`GitHub API error: ${response.statusText}`);
          }

          const diff = await response.text();

          // Parse diff stats
          const lines = diff.split('\n');
          const addedLines = lines.filter(l => l.startsWith('+') && !l.startsWith('+++')).length;
          const removedLines = lines.filter(l => l.startsWith('-') && !l.startsWith('---')).length;
          const filesChanged = lines.filter(l => l.startsWith('diff --git')).length;

          return {
            content: [{
              type: 'text',
              text: diff.substring(0, 10000) + (diff.length > 10000 ? '\n... (truncated)' : ''),
            }],
            ui: {
              viewType: 'code-diff',
              title: args.pr ? `PR #${args.pr}` : `${args.base} → ${args.head}`,
              description: `${filesChanged} file(s) changed: +${addedLines} -${removedLines}`,
              metadata: {
                source: 'github',
                owner: args.owner,
                repo: args.repo,
                filesChanged,
                additions: addedLines,
                deletions: removedLines,
                timestamp: new Date().toISOString(),
              },
              diff: diff,
            },
          };
        } catch (error) {
          return {
            content: [{ type: 'text', text: `Error: ${error instanceof Error ? error.message : 'Unknown error'}` }],
            isError: true,
          };
        }
      }
    );

    // Register get_repo_info tool
    this.registerTool(
      {
        name: 'github_get_repo_info',
        description: 'Get detailed repository information and metadata',
        inputSchema: {
          type: 'object',
          properties: {
            owner: {
              type: 'string',
              description: 'Repository owner',
            },
            repo: {
              type: 'string',
              description: 'Repository name',
            },
          },
          required: ['owner', 'repo'],
        },
      },
      async (args) => {
        const token = await this.getToken();

        try {
          const response = await fetch(
            `${GITHUB_API_BASE}/repos/${args.owner}/${args.repo}`,
            {
              headers: token ? {
                'Authorization': `Bearer ${token}`,
                'Accept': 'application/vnd.github.v3+json',
              } : {
                'Accept': 'application/vnd.github.v3+json',
              },
            }
          );

          if (!response.ok) {
            throw new Error(`GitHub API error: ${response.statusText}`);
          }

          const repo = await response.json();

          return {
            content: [{
              type: 'text',
              text: JSON.stringify({
                name: repo.name,
                full_name: repo.full_name,
                description: repo.description,
                url: repo.html_url,
                stars: repo.stargazers_count,
                forks: repo.forks_count,
                watchers: repo.watchers_count,
                language: repo.language,
                default_branch: repo.default_branch,
                open_issues: repo.open_issues_count,
                license: repo.license?.name,
                topics: repo.topics,
                created: repo.created_at,
                updated: repo.updated_at,
                pushed: repo.pushed_at,
                owner: repo.owner.login,
                is_fork: repo.fork,
                is_private: repo.private,
                has_wiki: repo.has_wiki,
                has_pages: repo.has_pages,
              }, null, 2),
            }],
            ui: {
              viewType: 'json-tree',
              title: `${args.owner}/${args.repo}`,
              description: repo.description || 'GitHub Repository',
              metadata: {
                source: 'github',
                owner: args.owner,
                repo: args.repo,
                stars: repo.stargazers_count,
                forks: repo.forks_count,
                language: repo.language,
                timestamp: new Date().toISOString(),
              },
              data: repo,
            },
          };
        } catch (error) {
          return {
            content: [{ type: 'text', text: `Error: ${error instanceof Error ? error.message : 'Unknown error'}` }],
            isError: true,
          };
        }
      }
    );
  }

  private async getToken(): Promise<string | null> {
    // Request token from main thread
    return new Promise((resolve) => {
      const requestId = Date.now().toString();
      
      const handler = (event: MessageEvent) => {
        if (event.data?.type === 'token:response' && event.data?.requestId === requestId) {
          self.removeEventListener('message', handler);
          resolve(event.data.token);
        }
      };

      self.addEventListener('message', handler);
      
      self.postMessage({
        type: 'token:request',
        requestId,
        key: 'github_token',
      });

      // Timeout after 1 second
      setTimeout(() => {
        self.removeEventListener('message', handler);
        resolve(null);
      }, 1000);
    });
  }
}

// Setup the worker
setupWorkerServer(GitHubServer);
