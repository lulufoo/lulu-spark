const zh={nav0:"下载",nav1:"为什么",nav2:"Agent 接入",nav4:"Markdown",
h1:"留下让你成长的那部分",sub:"项目进了 Linear，团队知识进了 Notion，你和 Agent 协作中学到的东西、想通的道理，沉淀到 Spark。",
dl:"下载 macOS 版",c_t:"只留对你有用的",c_p:"技术上的经验、对话里的结论、值得回头看的想法，留下；进度和待办，不归这里。",
s_t:"Agent 自己会存",s_p:"两步接好，Agent 会自己把值得留的内容存进 Spark。",st1_t:"安装技能",st1_p:"让 Agent 知道该把什么存进 Spark、什么时候存。",st1_l:"在 GitHub 查看技能 →",st2_t:"连接 MCP",s_p2:"当然，你也可以随时自己存。",
r_t:"任何 Agent 都能调用",r_p:"Agent 通过 MCP 搜索 Spark，学到的东西成为下一次的起点。",l1:"协作",l2:"保存",l4:"调用",
a_t:"问问 Spark AI",a_p:"Spark AI 基于你的笔记回答问题、帮你整理，每个回答都附出处。",
a_q:"把我的入职笔记合并成一页",a_a:"好了，3 篇笔记已合并成一页，去掉了重复内容，并保留了原笔记链接。",a_c1:"搭好本地环境",a_c2:"读一遍架构概览",a_c3:"结对完成第一个小改动",a_s1:"入职.md",a_s2:"2 环境搭建踩坑",a_s3:"3 第一周的问题",a_in:"问问 Lulu Spark…",
m_t:"纯 Markdown，存在你自己的电脑上",m_p:"每条笔记都是你选定文件夹里的 .md 文件，不被任何工具锁定。",
e_t:"让每次协作，都让你更进一步。",foot:"© 2026 Lulu Spark · 和 AI Agent 一起学到的，存在本地"};
const els=[...document.querySelectorAll("[data-i]")],en={};els.forEach(e=>en[e.dataset.i]=e.innerHTML);
function set(l){const d=l==="zh-CN"?zh:en;els.forEach(e=>{if(d[e.dataset.i])e.innerHTML=d[e.dataset.i]});document.documentElement.lang=l;
document.getElementById("langl").textContent=l==="zh-CN"?"简体中文":"EN";localStorage.setItem("lang",l);}
const q=new URLSearchParams(location.search).get("lang");let cur=q||localStorage.getItem("lang")||"en";set(cur);
const lm=document.getElementById("lm");document.getElementById("lang").onclick=e=>{e.stopPropagation();lm.classList.toggle("open")};lm.querySelectorAll("a").forEach(a=>a.onclick=()=>{cur=a.dataset.l;set(cur);lm.classList.remove("open")});document.onclick=()=>lm.classList.remove("open");
