"""One invariant over random expressions, needing no oracle:

   an expression's value must not depend on the route it travels.

Each generated expression E is echoed five ways -- written out directly,
returned from a function, bound by let(), stored in a one-element vector and
indexed back out, and passed through the identity function -- plus once via a
top-level assignment. All six ECHO lines must be character-identical. A
difference is a scoping, copying or evaluation-order bug in the evaluator.
"""
import os, random, subprocess, sys
B = os.environ.get(
    "SCADFORGE",
    os.path.join(os.path.dirname(os.path.dirname(os.path.dirname(
        os.path.abspath(__file__)))), "target", "release", "scadforge"),
)
W = os.path.dirname(os.path.abspath(__file__))

NUM = ["0", "1", "-1", "2.5", "-0.25", "1e3", "1e-3", "0.1", "3", "7", "100",
       "1/3", "-0", "1e300", "1e-300"]
STR = ['"a"', '"hello world"', '""', '"tab\\there"', '"q\\"q"', '"\\\\"', '"\\u00e9"']
ATOM = NUM + STR + ["true", "false", "undef", "[]", '[1,2,3]', '[[1,2],[3,4]]',
                    "[0:3]", "[0:0.5:2]", "[3:-1:0]", '["a","b"]',
                    '"abc"[1]', "[1,2,3][1]", "[[1,0],[0,1]]", "version()",
                    "[undef, 1]", "[[]]", '[1,"a",true,undef]', "rec(3)"]
UN = ["-", "!", "+"]
BIN = ["+", "-", "*", "/", "%", "<", "<=", ">", ">=", "==", "!=", "&&", "||"]
FN1 = ["abs", "sign", "sin", "cos", "tan", "acos", "asin", "atan", "exp", "ln",
       "log", "sqrt", "ceil", "floor", "round", "len", "norm", "str", "chr",
       "ord", "is_undef", "is_bool", "is_num", "is_string", "is_list", "concat"]
FN2 = ["min", "max", "pow", "atan2", "cross", "concat", "search", "str",
       "lookup"]
FN3 = ["str", "concat", "max", "min"]

def expr(r, d=0):
    if d > 3 or r.random() < 0.30:
        return r.choice(ATOM)
    k = r.randrange(10)
    if k == 0:
        return "(%s %s %s)" % (expr(r, d+1), r.choice(BIN), expr(r, d+1))
    if k == 1:
        return "(%s%s)" % (r.choice(UN), expr(r, d+1))
    if k == 2:
        return "(%s ? %s : %s)" % (expr(r, d+1), expr(r, d+1), expr(r, d+1))
    if k == 3:
        return "%s(%s)" % (r.choice(FN1), expr(r, d+1))
    if k == 4:
        return "%s(%s, %s)" % (r.choice(FN2), expr(r, d+1), expr(r, d+1))
    if k == 5:
        return "[%s]" % ", ".join(expr(r, d+1) for _ in range(r.randrange(1, 4)))
    if k == 6:
        return "(%s)[%s]" % (expr(r, d+1), r.choice(["0", "1", "-1", "9", "0.6"]))
    if k == 7:
        # Parenthesised: `let` is the lowest-precedence prefix form, so a bare
        # one cannot be an operand (`1 + let(a=1) a` is a syntax error, and
        # correctly so -- see the reference's precedence table).
        return "(let(zz = %s) (zz))" % expr(r, d+1)
    if k == 8:
        return "%s(%s, %s, %s)" % (r.choice(FN3), expr(r, d+1), expr(r, d+1), expr(r, d+1))
    src = r.choice(["[0:2]", "[1,2,3]", '"abc"', "[]", "[0:0.5:1]", "[[1,2],[3,4]]"])
    body = expr(r, d + 1)
    form = r.randrange(4)
    if form == 0:
        return "[for (zz = %s) %s]" % (src, body)
    if form == 1:
        return "[for (zz = %s) if (is_num(zz)) %s]" % (src, body)
    if form == 2:
        return "[for (zz = %s) each [%s, %s]]" % (src, body, body)
    return "[for (zz = %s) let(yy = %s) yy]" % (src, body)

def program(seed):
    r = random.Random(seed)
    e = expr(r)
    return (
        "function rec(n) = n <= 0 ? [] : concat([n], rec(n - 1));\n"
        "function f() = %s;\n"
        "function g(x) = x;\n"
        "module m(p) { echo(p); }\n"
        "tl = %s;\n"
        "echo(%s);\n"                      # direct
        "echo(f());\n"                     # through a function's body
        "echo(let(q = %s) q);\n"           # through a let binding
        "echo([%s][0]);\n"                 # into a vector and back out
        "echo(g(%s));\n"                   # through a parameter
        "echo(tl);\n"                      # through a top-level assignment
        "m(%s);\n"                         # through a module's parameter
        "for (v = [%s]) echo(v);\n"        # through a loop variable
        "cube(1);\n" % ((e,) * 8)
    )

ROUTES = ["direct", "function", "let", "vector", "parameter", "toplevel",
          "module", "loop"]

lo, hi = int(sys.argv[1]), int(sys.argv[2])
bad = skipped = ok = 0
for seed in range(lo, hi):
    src = program(seed)
    p = os.path.join(W, "e.scad")
    open(p, "w").write(src)
    r = subprocess.run([B, "-o", "e.stl", "--export-format", "asciistl", "e.scad"],
                       capture_output=True, text=True, cwd=W, timeout=120)
    lines = [l for l in (r.stderr or "").splitlines() if l.startswith("ECHO:")]
    if r.returncode != 0 or len(lines) != len(ROUTES):
        skipped += 1
        if r.returncode != 0 and "empty" not in (r.stderr or ""):
            print("SEED %d: rc=%d %s" % (seed, r.returncode, (r.stderr or "").strip()[:160]))
        continue
    if len(set(lines)) != 1:
        print("SEED %d: ROUTE CHANGED THE VALUE" % seed)
        for tag, l in zip(ROUTES, lines):
            print("   %-9s %s" % (tag, l))
        open(os.path.join(W, "bad_expr_%d.scad" % seed), "w").write(src)
        bad += 1
        continue
    ok += 1
print("expr seeds %d..%d  clean %d  skipped %d  BAD %d" % (lo, hi, ok, skipped, bad))
