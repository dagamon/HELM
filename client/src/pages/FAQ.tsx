import { useState, useEffect } from "react";
import { FileText, ChevronRight } from "lucide-react";
import { api } from "@/api/client";

interface FaqArticle {
  slug: string;
  title: string;
}

interface FaqArticleContent extends FaqArticle {
  content: string;
}

function renderMarkdown(content: string) {
  return content.split("\n").map((line, i) => {
    if (line.startsWith("## "))
      return (
        <h2 key={i} className="text-base font-semibold mt-4 mb-2">
          {line.slice(3)}
        </h2>
      );
    if (line.startsWith("# "))
      return (
        <h1 key={i} className="text-lg font-semibold mb-3">
          {line.slice(2)}
        </h1>
      );
    if (line.startsWith("- ")) {
      const rest = line.slice(2);
      const bold = rest.match(/^\*\*(.+?)\*\*\s*[-–]\s*(.+)$/);
      if (bold)
        return (
          <div key={i} className="flex gap-2 ml-4 mb-1">
            <span className="text-text-muted">•</span>
            <span>
              <strong>{bold[1]}</strong> – {bold[2]}
            </span>
          </div>
        );
      return (
        <div key={i} className="flex gap-2 ml-4 mb-1">
          <span className="text-text-muted">•</span>
          <span>{rest}</span>
        </div>
      );
    }
    const numbered = line.match(/^(\d+)\.\s(.+)$/);
    if (numbered)
      return (
        <div key={i} className="flex gap-2 ml-4 mb-1">
          <span className="text-text-muted">{numbered[1]}.</span>
          <span>{numbered[2]}</span>
        </div>
      );
    if (line.trim() === "") return <div key={i} className="h-2" />;
    return <p key={i} className="mb-1">{line}</p>;
  });
}

export function FAQ() {
  const [articles, setArticles] = useState<FaqArticle[]>([]);
  const [selectedSlug, setSelectedSlug] = useState<string | null>(null);
  const [article, setArticle] = useState<FaqArticleContent | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    api.listFaqArticles().then(setArticles).catch(() => setArticles([]));
  }, []);

  useEffect(() => {
    if (!selectedSlug) return;
    setLoading(true);
    api
      .getFaqArticle(selectedSlug)
      .then(setArticle)
      .catch(() => setArticle(null))
      .finally(() => setLoading(false));
  }, [selectedSlug]);

  if (articles.length === 0) {
    return (
      <div className="max-w-4xl mx-auto">
        <h1 className="text-lg font-semibold mb-2">FAQ</h1>
        <p className="text-text-muted text-sm">Документация в разработке</p>
      </div>
    );
  }

  return (
    <div className="max-w-6xl mx-auto space-y-6">
      <div>
        <h1 className="text-lg font-semibold">FAQ</h1>
        <p className="text-text-muted text-sm mt-1">Документация и справка</p>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-4 gap-6">
        <div className="space-y-1">
          {articles.map((a) => (
            <button
              key={a.slug}
              onClick={() => setSelectedSlug(a.slug)}
              className={`w-full flex items-center gap-2 px-3 py-2 rounded-lg text-sm text-left transition-colors ${
                selectedSlug === a.slug
                  ? "bg-surface-hover text-text font-medium"
                  : "text-text-muted hover:bg-surface-hover hover:text-text"
              }`}
            >
              <FileText className="w-4 h-4 shrink-0" />
              <span className="truncate">{a.title}</span>
              <ChevronRight className="w-3 h-3 ml-auto shrink-0 opacity-50" />
            </button>
          ))}
        </div>

        <div className="md:col-span-3 border border-border rounded-lg p-5">
          <h2 className="text-base font-semibold mb-4">
            {selectedSlug
              ? articles.find((a) => a.slug === selectedSlug)?.title ?? "Статья"
              : "Выберите статью"}
          </h2>
          {!selectedSlug ? (
            <p className="text-text-muted text-sm">Выберите статью из списка слева</p>
          ) : loading ? (
            <p className="text-text-muted text-sm">Загрузка...</p>
          ) : article?.content ? (
            <div className="text-sm space-y-0.5">{renderMarkdown(article.content)}</div>
          ) : (
            <p className="text-text-muted text-sm">Статья не найдена</p>
          )}
        </div>
      </div>
    </div>
  );
}
