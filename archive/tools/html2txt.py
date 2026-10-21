"""Strip an archived HTML file to readable text. Stdlib only; run with -I."""
import html.parser
import re
import sys

DROP = {"script", "style", "noscript", "svg", "head"}
BREAK = {"p", "div", "br", "li", "tr", "h1", "h2", "h3", "h4", "h5", "h6",
         "section", "article", "blockquote", "table"}


class Text(html.parser.HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.out = []
        self.depth = 0

    def handle_starttag(self, tag, attrs):
        if tag in DROP:
            self.depth += 1
        elif tag in BREAK:
            self.out.append("\n")

    def handle_endtag(self, tag):
        if tag in DROP and self.depth:
            self.depth -= 1
        elif tag in BREAK:
            self.out.append("\n")

    def handle_data(self, data):
        if not self.depth:
            self.out.append(data)


def main(path):
    with open(path, "rb") as fh:
        raw = fh.read()
    parser = Text()
    parser.feed(raw.decode("utf-8", "replace"))
    text = "".join(parser.out)
    text = re.sub(r"[ \t ]+", " ", text)
    text = re.sub(r"\n\s*\n\s*\n+", "\n\n", text)
    sys.stdout.write("\n".join(line.strip() for line in text.splitlines()).strip() + "\n")


if __name__ == "__main__":
    main(sys.argv[1])
