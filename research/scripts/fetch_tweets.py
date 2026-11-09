import json,sys,re,html,urllib.request,urllib.parse,concurrent.futures as cf
base=sys.argv[1]
P=json.load(open(base+"/projects.json"))
def get(u,method="GET"):
    r=urllib.request.Request(u,method=method,headers={"User-Agent":"Mozilla/5.0"})
    return urllib.request.urlopen(r,timeout=20)
class NoRedir(urllib.request.HTTPRedirectHandler):
    def redirect_request(self,*a,**k): return None
op=urllib.request.build_opener(NoRedir)
def resolve(t):
    try:
        op.open(urllib.request.Request(t,method="HEAD",headers={"User-Agent":"curl/8"}),timeout=15)
    except urllib.error.HTTPError as e:
        return e.headers.get("location") or t
    except Exception: return t
    return t
def work(p):
    out=dict(p)
    try:
        h=json.load(get("https://publish.twitter.com/oembed?omit_script=1&url="+urllib.parse.quote(p["demo_url"])))["html"]
        links=re.findall(r'href="(https://t\.co/[^"]+)"',h)
        txt=re.sub(r'<br>','\n',h); txt=re.sub(r'<[^>]+>','',txt); out["tweet"]=html.unescape(txt).strip()
        out["links"]=[resolve(l) for l in dict.fromkeys(links)]
    except Exception as e: out["err"]=str(e)[:100]
    return out
with cf.ThreadPoolExecutor(8) as ex: R=list(ex.map(work,P))
json.dump(R,open(base+"/tweets.json","w"),indent=1)
print(sum("tweet" in r for r in R),"ok;",sum("err" in r for r in R),"err")
