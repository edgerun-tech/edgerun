// Generate conformance dashboard HTML from proto files and test results.
//
// Usage: go run ./cmd/generate-dashboard > dashboard/conformance.html
package main

import (
	"bufio"
	"fmt"
	"os"
	"path/filepath"
	"regexp"
	"strings"
)

type Item struct {
	Domain    string
	Name      string
	Kind      string // "enum", "message", "field", "enum_value"
	Tested    bool
	Tests     []string
	SourceFile string
}

func parseProtoFile(path string) []Item {
	data, _ := os.ReadFile(path)
	text := string(data)
	domain := strings.TrimSuffix(filepath.Base(path), ".proto")

	var items []Item

	// Enum values
	enumRe := regexp.MustCompile(`^\s*(\w+)\s*=\s*(\d+)`)
	enumNameRe := regexp.MustCompile(`^\s*enum\s+(\w+)`)
	inEnum := ""
	scanner := bufio.NewScanner(strings.NewReader(text))
	for scanner.Scan() {
		line := scanner.Text()
		if m := enumNameRe.FindStringSubmatch(line); len(m) > 1 {
			inEnum = m[1]
		}
		if inEnum != "" {
			if m := enumRe.FindStringSubmatch(line); len(m) > 2 {
				name := m[1]
				if name != strings.ToUpper(inEnum)+"_UNSPECIFIED" && m[2] != "0" {
					items = append(items, Item{
						Domain: domain, Name: inEnum + "." + name,
						Kind: "enum_value", SourceFile: filepath.Base(path),
					})
				}
			}
			if strings.Contains(line, "}") {
				inEnum = ""
			}
		}
		// Messages
		if m := regexp.MustCompile(`^\s*message\s+(\w+)`).FindStringSubmatch(line); len(m) > 1 {
			items = append(items, Item{
				Domain: domain, Name: m[1], Kind: "message",
				SourceFile: filepath.Base(path),
			})
		}
	}
	return items
}

func main() {
	// Collect all proto files
	var allItems []Item
	filepath.Walk("proto/edgerun/v0", func(path string, info os.FileInfo, err error) error {
		if err != nil || !strings.HasSuffix(path, ".proto") {
			return nil
		}
		allItems = append(allItems, parseProtoFile(path)...)
		return nil
	})

	// Count
	tested := 0
	for _, it := range allItems {
		if it.Tested {
			tested++
		}
	}

	total := len(allItems)
	pct := 0.0
	if total > 0 {
		pct = float64(tested) / float64(total) * 100
	}

	// Generate HTML
	var L []string
	L = append(L, fmt.Sprintf(`<!DOCTYPE html><html><head><meta charset="utf-8">
<title>Conformance Dashboard</title>
<style>
body{font-family:system-ui,sans-serif;margin:0;padding:20px;background:#0d1117;color:#c9d1d9}
h1{color:#58a6ff}
.summary{display:flex;gap:20px;margin:20px 0}
.card{background:#161b22;border:1px solid #30363d;border-radius:8px;padding:20px;min-width:160px}
.card .num{font-size:2em;font-weight:bold}
.card.tested .num{color:#3fb950}
.card.untested .num{color:#f85149}
.card.total .num{color:#58a6ff}
input[type=text]{background:#0d1117;border:1px solid #30363d;color:#c9d1d9;padding:8px 12px;border-radius:6px;width:300px}
.filter-btn{background:#21262d;border:1px solid #30363d;color:#c9d1d9;padding:6px 14px;border-radius:6px;cursor:pointer;margin-left:8px}
.filter-btn.active{background:#1f6feb;border-color:#1f6feb}
section{margin:10px 0}
section h3{cursor:pointer;color:#58a6ff}
.item{display:flex;align-items:center;padding:4px 8px;border-radius:4px;font-size:14px}
.item.tested{color:#3fb950}
.item.untested{color:#f85149}
.item .icon{width:16px;margin-right:8px}
.item .kind{background:#30363d;padding:2px 6px;border-radius:3px;font-size:11px;margin-left:8px}
</style></head><body>
<h1>🔬 Conformance Dashboard</h1>

<div class="summary">
  <div class="card total"><div class="num">`+fmt.Sprintf("%d", total)+`</div><div>Total Items</div></div>
  <div class="card tested"><div class="num">`+fmt.Sprintf("%d", tested)+`</div><div>Tested</div></div>
  <div class="card untested"><div class="num">`+fmt.Sprintf("%d", total-tested)+`</div><div>Untested</div></div>
  <div class="card total"><div class="num">`+fmt.Sprintf("%.1f%%", pct)+`</div><div>Coverage</div></div>
</div>

<div>
  <input type="text" id="search" placeholder="Search items..." oninput="filter()">
  <button class="filter-btn active" onclick="setFilter('all')">All</button>
  <button class="filter-btn" onclick="setFilter('tested')">Tested</button>
  <button class="filter-btn" onclick="setFilter('untested')">Untested</button>
</div>

<div id="items">`)

	// Group by domain
	domains := make(map[string][]Item)
	for _, it := range allItems {
		domains[it.Domain] = append(domains[it.Domain], it)
	}

	for domain, items := range domains {
		L = append(L, fmt.Sprintf(`<section><h3 onclick="this.parentElement.classList.toggle('collapsed')">%s (%d items)</h3>`, domain, len(items)))
		for _, it := range items {
			status := "untested"
			icon := "○"
			if it.Tested {
				status = "tested"
				icon = "●"
			}
			L = append(L, fmt.Sprintf(`<div class="item %s" data-status="%s"><span class="icon">%s</span><span>%s</span><span class="kind">%s</span></div>`,
				status, status, icon, it.Name, it.Kind))
		}
		L = append(L, `</section>`)
	}

	L = append(L, `</div>
<script>
let currentFilter='all';
function setFilter(f){currentFilter=f;document.querySelectorAll('.filter-btn').forEach(b=>b.classList.remove('active'));
document.querySelectorAll('.filter-btn').forEach(b=>{if(b.textContent.toLowerCase()===f)b.classList.add('active')});filter()}
function filter(){const q=document.getElementById('search').value.toLowerCase();
document.querySelectorAll('.item').forEach(el=>{const name=el.textContent.toLowerCase();
const matchQ=!q||name.includes(q);
const matchF=currentFilter==='all'||el.dataset.status===currentFilter;
el.style.display=(matchQ&&matchF)?'':'none'})}
document.addEventListener('keydown',e=>{if(e.key==='/'){e.preventDefault();document.getElementById('search').focus()}})
</script></body></html>`)

	fmt.Println(strings.Join(L, "\n"))
}
