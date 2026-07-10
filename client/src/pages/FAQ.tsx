import { useEffect, useMemo, useRef, useState } from "react";
import { BookOpen, ChevronDown, Clipboard, ClipboardCheck, FileText, Hash } from "lucide-react";
import faqIndexRaw from "../../../faq/index.json?raw";

interface FaqArticle {
  slug: string;
  title: string;
  content: string;
  section: string | null;
}

interface FaqIndex {
  sections?: {
    user?: {
      title: string;
      items: string[];
    }[];
  };
}

interface Heading {
  level: 2 | 3;
  text: string;
  id: string;
}

type TextBlock = { type: "text"; lines: string[] };
type CodeBlockData = { type: "code"; lang: string; code: string };
type Block = TextBlock | CodeBlockData;

const articleModules = import.meta.glob("../../../faq/*.md", {
  eager: true,
  query: "?raw",
  import: "default",
}) as Record<string, string>;

const slugify = (text: string) =>
  text
    .toLowerCase()
    .replace(/\s+/g, "-")
    .replace(/[^\w-]/g, "")
    .replace(/-+/g, "-")
    .replace(/^-|-$/g, "");

function titleFromMarkdown(slug: string, content: string) {
  const titleLine = content.split("\n").find((line) => line.startsWith("# "));
  return titleLine?.slice(2).trim() || slug.replace(/-/g, " ").replace(/\b\w/g, (c) => c.toUpperCase());
}

function extractSlug(path: string) {
  return path.split("/").pop()?.replace(/\.md$/, "") ?? path;
}

function loadArticles(): FaqArticle[] {
  const index = JSON.parse(faqIndexRaw) as FaqIndex;
  const bySlug = new Map<string, { slug: string; content: string; title: string }>();

  for (const [path, content] of Object.entries(articleModules)) {
    const slug = extractSlug(path);
    bySlug.set(slug, { slug, content, title: titleFromMarkdown(slug, content) });
  }

  const articles: FaqArticle[] = [];
  const seen = new Set<string>();

  for (const section of index.sections?.user ?? []) {
    for (const slug of section.items) {
      const article = bySlug.get(slug);
      if (!article) continue;
      articles.push({ ...article, section: section.title });
      seen.add(slug);
    }
  }

  for (const article of [...bySlug.values()].sort((a, b) => a.slug.localeCompare(b.slug))) {
    if (!seen.has(article.slug)) articles.push({ ...article, section: null });
  }

  return articles;
}

function extractHeadings(content: string): Heading[] {
  return content
    .split("\n")
    .filter((line) => line.startsWith("## ") || line.startsWith("### "))
    .map((line) => {
      const level = line.startsWith("### ") ? 3 : 2;
      const text = line.slice(level === 3 ? 4 : 3).trim();
      return { level, text, id: slugify(text) };
    });
}

function parseBlocks(content: string): Block[] {
  const lines = content.split("\n");
  const blocks: Block[] = [];
  let textLines: string[] = [];
  let i = 0;

  while (i < lines.length) {
    const line = lines[i];
    if (line.startsWith("```")) {
      if (textLines.length > 0) {
        blocks.push({ type: "text", lines: [...textLines] });
        textLines = [];
      }
      const lang = line.slice(3).trim();
      const codeLines: string[] = [];
      i++;
      while (i < lines.length && !lines[i].startsWith("```")) {
        codeLines.push(lines[i]);
        i++;
      }
      blocks.push({ type: "code", lang, code: codeLines.join("\n") });
    } else {
      textLines.push(line);
    }
    i++;
  }

  if (textLines.length > 0) blocks.push({ type: "text", lines: textLines });
  return blocks;
}

function renderInline(text: string): React.ReactNode {
  const parts = text.split(/(\*\*[^*]+\*\*|`[^`]+`)/g);
  if (parts.length === 1) return text;

  return (
    <>
      {parts.map((part, i) => {
        if (part.startsWith("**") && part.endsWith("**")) return <strong key={i}>{part.slice(2, -2)}</strong>;
        if (part.startsWith("`") && part.endsWith("`")) {
          return (
            <code key={i} className="px-1 py-0.5 rounded text-xs bg-surface-hover font-mono text-accent">
              {part.slice(1, -1)}
            </code>
          );
        }
        return part;
      })}
    </>
  );
}

function MarkdownTable({ lines }: { lines: string[] }) {
  const rows = lines.filter((line) => !line.match(/^\|\s*[-:]+[\s|-]*\|/));
  const parsed = rows.map((line) => line.split("|").slice(1, -1).map((cell) => cell.trim()));
  const [header, ...body] = parsed;

  if (!header) return null;

  return (
    <div className="overflow-x-auto my-3 rounded-lg border border-border">
      <table className="w-full text-sm border-collapse">
        <thead>
          <tr className="border-b border-border bg-surface-hover/60">
            {header.map((cell, i) => (
              <th key={i} className="text-left px-3 py-2 font-medium text-text">
                {renderInline(cell)}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {body.map((row, rowIndex) => (
            <tr key={rowIndex} className="border-b border-border/60 last:border-0">
              {row.map((cell, cellIndex) => (
                <td key={cellIndex} className="px-3 py-2 text-text-muted">
                  {renderInline(cell)}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function renderTextBlock(lines: string[]) {
  const result: React.ReactNode[] = [];
  let i = 0;

  while (i < lines.length) {
    const line = lines[i];

    if (line.trimStart().startsWith("|")) {
      const tableLines: string[] = [];
      while (i < lines.length && lines[i].trimStart().startsWith("|")) {
        tableLines.push(lines[i]);
        i++;
      }
      result.push(<MarkdownTable key={`table-${i}`} lines={tableLines} />);
      continue;
    }

    if (line.startsWith("### ")) {
      const text = line.slice(4).trim();
      result.push(
        <h3 id={slugify(text)} key={i} className="text-sm font-semibold mt-5 mb-1.5 text-text scroll-mt-6">
          {renderInline(text)}
        </h3>,
      );
    } else if (line.startsWith("## ")) {
      const text = line.slice(3).trim();
      result.push(
        <h2 id={slugify(text)} key={i} className="text-base font-semibold mt-6 mb-2 text-text scroll-mt-6">
          {renderInline(text)}
        </h2>,
      );
    } else if (line.startsWith("# ")) {
      result.push(
        <h1 key={i} className="text-lg font-semibold mb-3 text-text">
          {renderInline(line.slice(2).trim())}
        </h1>,
      );
    } else if (line.startsWith("> ")) {
      result.push(
        <blockquote key={i} className="border-l-2 border-accent/60 pl-3 text-text-muted italic my-2">
          {renderInline(line.slice(2))}
        </blockquote>,
      );
    } else if (line.trim() === "---") {
      result.push(<hr key={i} className="border-border my-4" />);
    } else if (line.startsWith("- ")) {
      result.push(
        <div key={i} className="flex gap-2 ml-4 mb-1">
          <span className="text-text-muted mt-0.5 shrink-0">•</span>
          <span>{renderInline(line.slice(2))}</span>
        </div>,
      );
    } else if (/^\d+\.\s/.test(line)) {
      const match = line.match(/^(\d+)\.\s(.+)$/);
      if (match) {
        result.push(
          <div key={i} className="flex gap-2 ml-4 mb-1">
            <span className="text-text-muted shrink-0">{match[1]}.</span>
            <span>{renderInline(match[2])}</span>
          </div>,
        );
      }
    } else if (line.trim() === "") {
      result.push(<div key={i} className="h-1.5" />);
    } else {
      result.push(
        <p key={i} className="mb-1 leading-relaxed text-text-muted">
          {renderInline(line)}
        </p>,
      );
    }
    i++;
  }

  return result;
}

function CodeBlock({ lang, code }: { lang: string; code: string }) {
  const [copied, setCopied] = useState(false);

  const copy = () => {
    void navigator.clipboard.writeText(code);
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1500);
  };

  return (
    <div className="relative my-3 rounded-lg overflow-hidden border border-border">
      <div className="flex items-center justify-between px-3 py-1.5 bg-surface-hover border-b border-border">
        <span className="text-xs text-text-muted font-mono">{lang || "code"}</span>
        <button
          type="button"
          onClick={copy}
          className="inline-flex items-center gap-1.5 text-xs text-text-muted hover:text-text transition-colors"
        >
          {copied ? <ClipboardCheck className="w-3 h-3" /> : <Clipboard className="w-3 h-3" />}
          {copied ? "Copied" : "Copy"}
        </button>
      </div>
      <pre className="overflow-x-auto p-3 text-xs font-mono bg-black/40 text-text leading-relaxed">
        <code>{code}</code>
      </pre>
    </div>
  );
}

function ArticleContent({ content }: { content: string }) {
  return (
    <div className="text-sm space-y-0.5">
      {parseBlocks(content).map((block, i) =>
        block.type === "code" ? (
          <CodeBlock key={i} lang={block.lang} code={block.code} />
        ) : (
          <div key={i}>{renderTextBlock(block.lines)}</div>
        ),
      )}
    </div>
  );
}

function Sidebar({
  articles,
  selectedSlug,
  headings,
  activeHeadingId,
  onSelect,
}: {
  articles: FaqArticle[];
  selectedSlug: string | null;
  headings: Heading[];
  activeHeadingId: string | null;
  onSelect: (slug: string) => void;
}) {
  let lastSection: string | null = null;

  return (
    <div className="space-y-0.5">
      {articles.map((article) => {
        const selected = selectedSlug === article.slug;
        const hasHeadings = selected && headings.length > 0;
        const showSection = article.section !== lastSection;
        lastSection = article.section;

        return (
          <div key={article.slug}>
            {showSection && article.section && (
              <div className="px-3 pt-4 pb-1 text-xs font-semibold text-text-tertiary uppercase tracking-wide select-none">
                {article.section}
              </div>
            )}
            <button
              type="button"
              onClick={() => onSelect(article.slug)}
              className={`faq-sidebar-row w-full flex items-center gap-2 px-3 py-2 rounded-lg text-sm text-left ${
                selected
                  ? "bg-surface-hover text-text font-medium"
                  : "text-text-muted hover:bg-surface-hover hover:text-text"
              }`}
            >
              <FileText className="w-4 h-4 shrink-0" />
              <span className="truncate flex-1">{article.title}</span>
              {hasHeadings && <ChevronDown className="w-3 h-3 shrink-0 opacity-60" />}
            </button>

            {hasHeadings && (
              <div className="faq-outline-enter ml-3 mt-0.5 mb-1 pl-3 border-l border-border/70 space-y-0.5">
                {headings.map((heading) => (
                  <button
                    key={heading.id}
                    type="button"
                    onClick={() => document.getElementById(heading.id)?.scrollIntoView({ behavior: "smooth" })}
                    className={`faq-sidebar-row w-full flex items-center gap-1.5 text-left rounded px-2 py-1 ${
                      heading.level === 3 ? "ml-3 text-xs" : "text-xs"
                    } ${
                      activeHeadingId === heading.id
                        ? "text-text bg-surface-hover"
                        : "text-text-muted hover:text-text hover:bg-surface-hover/70"
                    }`}
                  >
                    <Hash className={`shrink-0 opacity-50 ${heading.level === 2 ? "w-3 h-3" : "w-2.5 h-2.5"}`} />
                    <span className="truncate">{heading.text}</span>
                  </button>
                ))}
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}

export function FAQ() {
  const articles = useMemo(loadArticles, []);
  const [selectedSlug, setSelectedSlug] = useState<string | null>(() => articles[0]?.slug ?? null);
  const [activeHeadingId, setActiveHeadingId] = useState<string | null>(null);
  const [previousArticle, setPreviousArticle] = useState<FaqArticle | null>(null);
  const switchTimerRef = useRef<number | null>(null);
  const contentRef = useRef<HTMLDivElement>(null);
  const selectedArticle = articles.find((article) => article.slug === selectedSlug) ?? null;
  const headings = selectedArticle ? extractHeadings(selectedArticle.content) : [];

  useEffect(() => {
    if (selectedSlug || articles.length === 0) return;
    setSelectedSlug(articles[0].slug);
  }, [articles, selectedSlug]);

  useEffect(() => {
    if (!contentRef.current || headings.length === 0) return;

    const headingEls = headings.map((heading) => document.getElementById(heading.id)).filter(Boolean) as HTMLElement[];
    const observer = new IntersectionObserver(
      (entries) => {
        const visible = entries.filter((entry) => entry.isIntersecting);
        if (visible.length > 0) setActiveHeadingId(visible[0].target.id);
      },
      { rootMargin: "-10% 0px -80% 0px", threshold: 0 },
    );

    headingEls.forEach((element) => observer.observe(element));
    return () => observer.disconnect();
  }, [headings, selectedArticle]);

  useEffect(() => {
    return () => {
      if (switchTimerRef.current !== null) window.clearTimeout(switchTimerRef.current);
    };
  }, []);

  const selectArticle = (slug: string) => {
    if (slug === selectedSlug) return;
    if (switchTimerRef.current !== null) window.clearTimeout(switchTimerRef.current);

    setPreviousArticle(selectedArticle);
    setSelectedSlug(slug);
    setActiveHeadingId(null);
    switchTimerRef.current = window.setTimeout(() => {
      setPreviousArticle(null);
      switchTimerRef.current = null;
    }, 420);
  };

  return (
    <div className="faq-page-enter max-w-6xl mx-auto space-y-6">
      <div>
        <h1 className="text-lg font-semibold">FAQ</h1>
        <p className="text-text-muted text-sm mt-1">User guide and common workflows</p>
      </div>

      <div className="flex gap-2 border-b border-border">
        <button
          type="button"
          className="flex items-center gap-2 px-4 py-2 text-sm font-medium border-b-2 -mb-px border-accent text-text"
        >
          <BookOpen className="w-4 h-4" />
          User Guide
        </button>
      </div>

      {articles.length === 0 ? (
        <div className="rounded-lg border border-border bg-surface/70 p-12 text-center text-text-muted text-sm">
          Documentation in progress
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-4 gap-6 items-start">
          <div className="md:sticky md:top-6">
            <Sidebar
              articles={articles}
              selectedSlug={selectedSlug}
              headings={headings}
              activeHeadingId={activeHeadingId}
              onSelect={selectArticle}
            />
          </div>

          <div className="md:col-span-3" ref={contentRef}>
            <div className="faq-card rounded-lg border border-border bg-surface/70 overflow-hidden">
              <div className="px-5 py-4 border-b border-border">
                <h2 className="text-base font-semibold">{selectedArticle?.title ?? "Select an article"}</h2>
              </div>
              <div className="faq-content-stage p-5">
                {previousArticle && previousArticle.slug !== selectedArticle?.slug && (
                  <div className="faq-content-layer-exit faq-content-exit">
                    <ArticleContent content={previousArticle.content} />
                  </div>
                )}
                <div
                  key={selectedArticle?.slug ?? "empty"}
                  className="faq-content-enter"
                >
                  {selectedArticle ? (
                    <ArticleContent content={selectedArticle.content} />
                  ) : (
                    <p className="text-text-muted text-sm">Select an article from the list on the left</p>
                  )}
                </div>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
