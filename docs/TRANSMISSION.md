# The spherical planetary stack: what it is, and what building it found

Seven files in `scadforge/examples/transmission/` describe one machine:
spherical-bevel planetary slices bolted end to end on one polar axis, each
joint a facing pair of polar caps, with a swappable collar on each
equatorial band — a roller-chain sprocket on one, a regenerative axial-flux
machine on the next. The design table holds four rows; which of them a given
stack carries, and in what order, is configuration.

This document is the map between them. It is not a summary of their
headers — each file argues its own case at length and prints every number it
claims — but the part a reader cannot get from any single file: how the seven
fit together, which numbers cross a boundary, what was decided at each
boundary, and what assembling them actually found.

**Every number below is printed by the file it belongs to.** Running
`scadforge -o out.echo <file>.scad` reproduces it. Where a figure was
measured outside the files — a triangle count, an export timing — it says so.

---

## 1. The files

| file | what it is | tris | components | holes | flipped | T-junctions |
|---|---|---|---|---|---|---|
| `spherical_gear.scad` | the contract: design table, the spherical involute, the gates | 10,268 | 5 | 0 | 0 | 0 |
| `slice.scad` | one planetary row, any row | 116,464 (row 1) | 18 | 0 | 0 | 0 |
| `equator_band.scad` | the equator row's ring, plus an external spur port | 393,204 | 2 | 0 | 0 | 0 |
| `polar_cap.scad` | the end of the stack: crown coupling, thrust race, spline | 132,776 | 40 | 0 | 0 | 0 |
| `collar_chain.scad` | a sprocket collar for the register | 48,280 | 14 | 0 | 0 | 0 |
| `collar_em.scad` | a 24-slot / 28-pole axial-flux collar with a capacitor bank | 73,784 | 108 | 0 | 0 | 0 |
| `stack.scad` | the machine | 2,332,492 | 180 | 0 | 0 | 0 |

`slice.scad` takes the row as a parameter; the four rows come out at
116,464 / 237,360 / 291,216 / 407,952 triangles and 18 / 10 / 10 / 10
components.

Nothing in any of them uses `difference()` or `intersection()`. The members
share no volume, so the exported mesh is face for face what the file writes.

### The dependency graph

```
spherical_gear.scad            the contract; publishes sg_*()
   |
   +-- slice.scad              publishes sl_*(), sl_if_*()
   +-- polar_cap.scad          publishes pc_if_*()
   +-- equator_band.scad       publishes eb_if_*()
          |
          +-- collar_chain.scad   publishes cc_if_*()
          +-- collar_em.scad      publishes em_if_*()

stack.scad  uses all six
```

That graph is two levels deep, and **`use` did not work two levels deep.**
A used file's top-level constants are evaluated so its functions can see
them, and all the definitions landed in one flat table the evaluator walks
in order — but the table was built breadth-first, dependents before
dependencies. A part file opening with `M = sg_m();` was evaluated before
the contract's own constants were assigned, so every number it read came
back `undef`, with a warning naming a private spelling nobody wrote
(`Ignoring unknown variable '__use1__MODULE_MM'`). One level deep there is
nothing to order, which is why it had stood. The walk is now depth-first
and post-order.

---

## 2. The design, in one page

From `spherical_gear.scad`, and read from it by every other file:

- module **m = 1 mm**, pressure angle **phi = 25°**, backlash **jt = 0.05**
  modules per mesh
- one sun count **Ns = 46** at every row, so one sun pitch diameter 46 mm
- a fundamental sphere **R = 121 mm**, i.e. **Dref = 2R/m = 242**

and four rows `[name, Nr, D, Np, k]`:

| row | Nr | D | Np | k | gamma_s | L = Dm/2 |
|---|---|---|---|---|---|---|
| row 1 | 59 | 62 | 13 | 7 | 47.897° | 31 mm |
| row 2 | 143 | 146 | 73 | 3 | 18.365° | 73 mm |
| row 3 | 188 | 196 | 98 | 3 | 13.574° | 98 mm |
| equator | 242 | 242 | 154 | 3 | 10.958° | 121 mm |

The row closure identity is exact on all four: `gamma_s + 2 gamma_p =
gamma_r` to 1.4e-14, and the assembly gate `(Ns + Nr)/k` is an integer on
all four (15, 63, 78, 96). Every flank clearance comes out at the design
half-backlash of 0.025 mm.

### The spherical involute

The flank is the exact spherical involute, not a projected planar one:

```
sin(gamma_b) = sin(gamma) cos(phi)
```

and for a crown gear, gamma = 90, this collapses to `gamma_b = 90 - phi`
exactly. The equator row's ring is such a crown: its pitch cone is the
equatorial *plane*. The contract records this as a failed gate against the
original specification, which asked for a cylinder there, and it is a
failure of the specification rather than of the design.

### The shared-apex gate, and the stack

The contract searches every `D` from 40 to 1500 and every sun count, and
finds that one apex with one module and one sun count carries **at most one
exact latitudinal row beside an equator row**, and never more than three
below D = 900 even with no equator row. Three latitudinal rows plus the
equator on one apex has no solution at any buildable size. That is a failed
gate and it is reported as one.

The resolution is the stack: per-slice apexes, one shaft, one sun count, one
sun pitch diameter, **stepped cone angles** `arcsin(Ns/D_i)`. Section 4
below is about where those apexes actually go, because the obvious answer
does not work.

---

## 3. The interfaces, and the rule they all broke

Six interfaces cross file boundaries. Each is a set of `*_if_*()` functions.

| published by | read by | what crosses |
|---|---|---|
| `sg_*()` | everyone | m, phi, jt, Ns, Dref, R, the row table, EM counts, capacitor rating |
| `sl_if_*()` | `stack.scad` | apex, latitude, planet-axis cone, cone distances, ring and carrier extents |
| `eb_if_*()` | both collars, `stack.scad` | the register: floor, lands, groove, clearances, the bore a collar must cut, the band's own envelope |
| `cc_if_*()` | `stack.scad` | bore, tongue, hub width, sprocket tip, bolt circle, envelope |
| `em_if_*()` | `stack.scad` | seat, clamp width and wall, rib, split, bolt circle, envelope |
| `pc_if_*()` | `stack.scad` | z extent, radius, crown, bolt circle, spline |

**The rule, and it was broken four times:** a number two files have to agree
on is *read*, never copied. Every defect in section 5 is the same shape — one
file published a number, the other restated it from the published *text*, and
nothing ever compared the two.

The most expensive instance: `equator_band.scad` computed the bore, tongue
face and tongue width a collar must be built to, and printed them in its
report. Nothing could read them. `collar_chain.scad` restated all five
underlying numbers and happened to get them right. `collar_em.scad` restated
them from the same text and got the seat wrong by 21.2 mm.

---

## 4. Where the rows go

`slice.scad` publishes equation (15), which is the contract's section 4
written as arithmetic. A row's ring pitch circle is a latitude circle of the
fundamental sphere — `Nr_i = Dref sin(colat_i)`, so the circle's radius
`Nr_i m/2` equals `R sin(colat_i)` automatically — and the row's own apex
frame puts that circle at `L cos(gamma_r)` above the apex, so

```
z_apex = R cos(colat) - L cos(gamma_r),   colat = asin(Nr/Dref)
```

which gives apexes at **107.823, 82.893, 48.478 and 0**, the equator row's on
the sphere centre exactly as the contract says, and puts all four ring
circles on the sphere: 29.5² + 117.349² = 121², and so on for the other
three.

**And it cannot be built.** `stack.scad` section 6 evaluates it. Each row's
sun has a solid hub band — from its bore cone to its root cone, material at
every azimuth — so two hub bands sharing a point of the (r, z) half-plane is
a collision outright, with no question of whether teeth interleave. Sampling
one on a 13×13 grid and testing membership in the other:

```
row 1 sun into row 3 sun     109 of 169 points inside
row 2 sun into row 3 sun      47 of 169 points inside
```

and worse, the equator row's own shell runs from cone distance 90.75 to 121
about the sphere centre while every other row's ring circle lies at
|p| = 121 — *on that shell's outer surface*. The equator row fills the space
the other three rows' rings need.

So the sphere keeps the job it can do, which is the one the divisibility gate
needs: choosing the integer `Nr_i` selects the row's ring pitch radius. Where
the row sits on the axis is then a clearance question, and `stack.scad`
answers it by stacking:

```
station 0 is the bottom slice, apex at the origin;
above it a pole joint, then station 1, and so on.                    (16)
```

A **station** is a slice, optionally a band, optionally a collar — and the
station list is data, `[row index, collar code]`, so the machine is
configured rather than hard-coded. The default is three stations: an
equator-row slice with the chain collar, a second with the EM collar, and a
row-1 slice bare. That is the picture the machine is for — one slice taking
a chain, the next taking a motor, on the *same* register.

Every member of a slice is a spherical shell sector, so its z extreme is at
a corner of its (cone distance, colatitude) rectangle; the band and the
collar publish their z extents outright. (16) makes consecutive stations'
z intervals disjoint, separated by the mid-planes of the joints at
z = 156.016 and 420.656. **Two solids on opposite sides of a plane cannot
touch, whatever their radii**, so the whole between-station clearance
argument is one comparison per joint and it is exact. It is also why the EM
collar, whose stator lugs stand further out than anything else in the
machine at r = 228 mm, needs no special pleading: it sits inside its own
station's interval and that interval is private to the station.

The sun is 46 mm across at every station — `2 L sin(gamma_s) = Ns m`, with
difference 0 printed per station — so one cap and one spline serve the whole
stack.

### The pole joint, which is what makes it a stack

Between two slices sit **two polar caps, face to face**. That is what
`polar_cap.scad` was built for, and it is the one thing no part file can
supply, because what it needs is the second cap. Between them the pair make
a face coupling of two crowns, a thrust race, a socket round the floating
spline sleeve, and a bolted joint. The crowns interleave because the cap
indexes its own crown a quarter pitch: centres at `(j + ¼)p` map under the
mate's 180° turn to `−(j + ¼)p`, half a pitch away — onto this cap's space
centres.

The joint is drawn with a small **declared** gap rather than closed. At zero
the two webs and the two bolt rings would share faces, and shared faces are
a boolean; worse, two shells sharing vertices come back from the exporter as
*one* component, and the component count would stop being a check. The flank
clearance computed at zero gap is a lower bound for any positive gap,
because opening the joint moves each tooth toward the thinner part of the
facing space.

Both **end** caps face outward, which is what makes the stack a module
rather than a finished box: an end cap presents exactly the face an internal
joint presents — the same 138-tooth crown, the same 46 spline — so a second
stack bolts onto either end and the result is still a stack. It is also why
power enters and leaves at the poles: the pole is the only interface that
repeats.

### The price

A machine that fitted inside the fundamental sphere would be 242 mm tall.
This one is **606.7 mm**, a factor of 2.51. A stack of bevel sets with a
bolted pole joint between each pair is long, and that length is the cost of
the exactness, not a modelling artefact. It is stated in the file rather
than buried.

## 5. What assembling it found

Five defects, none visible from inside a part file.

**1. `use` two levels deep.** Described in section 1. Every number a part
read from the contract came back `undef` the moment a third file used the
part. Fixed in the kernel: the definition table is now a topological order
of the use graph.

**2. The EM collar clamped nothing.** Its seat was `R + off` with
`off = 24 mm` chosen to stand outboard of the band's toothed outer face. The
band's register lands crest at 123.5 and a collar bores 123.8; the EM
collar's seat was 145 — **21.2 mm outboard of the register, over the spur
teeth**. Two further errors came with it: an 18 mm clamp against a 20 mm
register, and an azimuthal keyway tongue where the band offers a
circumferential groove. All three are now read from `eb_if_*()`; the tongue
is a circumferential rib, the same feature the chain collar uses, so the two
collars interchange in fact and not only in claim. Its derived bore and the
chain collar's are now the same number, difference 0.

**3. The pin boss band was `beta ± 2·GPIN`** — a bare factor of two on the
pin's own cone. On row 1, where it was written, that is a 12° margin. On the
equator row, whose planets are 154 teeth, it is 36°, and the retainer came
out as a near-hemispherical shell reaching colatitude 123 — straight through
the band's register lands. It is now the pin's cone plus a declared 2 mm at
the mean cone distance, and the equator row's z extent drops from
−67.6…121.0 to −6.9…121.0.

**4. The band's register relief was 2 mm**, putting its spur flange 12 mm
above the groove centre. Both collars publish an envelope taller than that —
the chain collar's clamp lugs stand 19.2 mm off its centre line, the EM
collar's tabs 16.05. **Either one's lugs fouled the flange.** The relief is
now 10.5 mm: the flange stands 20.5 mm up, clearing the taller of the two by
1.3 mm. The register grows from 22 to 30.5 mm.

**5. The band's crown seat had no clearance at all** — and this one was
found by a different method, which is the point of it. The seat and the
crown ring's own back cone were cut from the *same rule* at `GHUB`, so
without an offset they are the same cone and the two members touch over the
whole annulus. The file said so deliberately: *"the seat fit is a CONTACT,
not a clearance."* As a statement about the joint that is right — a seat
carries load through contact. As a statement about the model it was the one
claim the file could not back up. Two coincident surfaces share no volume,
so *"the members share no volume"* stayed literally true, no printed
clearance could catch it, and exact contact is precisely the case the
export-time union is documented to fail on. `polar_cap.scad` draws the
identical feature — a crown ring seated on a back cone — with
`PC_SEAT_Z = 0.4` and always has. The band now takes the cap's answer.

What found it is `scadforge/tools/overlap.py`: an exact edge-crosses-face
test between every pair of bodies whose bounding boxes meet, with a ray cast
afterwards for the nesting case that crosses no surface at all. It is the
check every one of these files raises and then has to leave open, because
the component count cannot answer it — two shells that interpenetrate
without sharing a vertex still count as two — and past the merge budget the
union does not run. Results:

| part | bodies | segment-triangle tests | verdict |
|---|---|---|---|
| `collar_chain` | 14 | 43,032 | ALL DISJOINT |
| `polar_cap` | 40 | 4,301,290 | ALL DISJOINT |
| pole joint | 6 | 23,790,749 | ALL DISJOINT |
| `equator_band` | 2 | 223 | **OVERLAP FOUND** |
| `equator_band`, fixed | 2 | 111,768,502 | ALL DISJOINT |

Note the band's two test counts. 223 to find a crossing, because the search
stops at the first one; 111,768,502 to establish there is none. A negative
costs everything, which is why this is a tool you run deliberately and not
a check on every export.

So three of those claims are now *proved* rather than argued from printed
clearances, and the fourth was false and is now fixed and proved. The check belongs in the kernel — "are these two
shells disjoint" is what the union is for — and the script is the evidence
until it gets there.

**The first four share one pattern, and it is the lesson:** *a number that
two files have to agree on was published by one and copied by the other, and
the copy was never compared with the original.* An assembly is where the
comparison happens.

**The fifth is a different lesson and a harder one.** Nothing was copied and
nothing disagreed; one file made a decision, stated it, and printed the
residual. What it could not do was *check* it, because the thing it had
decided — exact contact — is invisible to every check the project had.
Volume cannot see it, the component count cannot see it, and the export-time
union is documented to fail on it. It took a check of a kind that did not
exist here before.

Until there was an assembly, none of the five was visible, and four of them
had been sitting in files that validate clean.

---

## 6. The drive, and the one thing the geometry does not settle

Each row is a planetary set governed by Willis,
`(w_sun − w_carrier)/(w_ring − w_carrier) = −Nr/Ns`, exact in the tooth
counts. Hold a slice's ring and that slice reduces by
`i = (Ns + Nr)/Ns`.

**What the stack as a whole does depends on one thing that is not modelled.**
The pole joint drawn here couples **cap to cap** — crown to crown, spline to
spline through the floating sleeve. Which member of its own slice each cap
is bolted to, the carrier or the sun, is nowhere in these files, and that
choice is what decides the machine. Both readings are printed, each with the
assumption it rests on:

**Series.** A cap bolted to the carrier on one side of a joint and carrying
the sun through on the other puts the slices in series, and the stack is one
compound reduction

```
i_stack = prod_i (Ns + Nr_i) / Ns^n
```

which for the default three stations is `1088640/12167 = 89.4748 : 1`, an
exact reduced rational. **The order does not matter**, and the file shows it
twice: every permutation of the station list gives the same numerator
(spread 0), and the physical half — the slices really can be bolted in any
order — holds because every station presents the same pole interface, the
same 46 spline and the same 138-tooth crown, which in turn holds because a
member's pitch *diameter* is `N m` at every cone distance.

**Differential.** Every cap taking the sun through, with the carriers tied
to one output, makes it a two-degree-of-freedom machine instead: fix any two
of {sun, carrier, one ring} and every other speed follows. Holding one ring
at a time then *selects*:

| row | (Ns+Nr)/Ns | reduced | value |
|---|---|---|---|
| row 1 | 105/46 | 105/46 | 2.28261 |
| row 2 | 189/46 | 189/46 | 4.10870 |
| row 3 | 234/46 | 117/23 | 5.08696 |
| equator | 288/46 | 144/23 | 6.26087 |

a **four-speed**, spread 2.74286, in steps of 1.800, 1.238, 1.231 — uneven,
and said so: the counts were chosen to make each row close exactly, not to
space the ratios.

With no ring held it is a torque split. Power balance with the sun held
gives `T_carrier = −Σ T_ring_i (Ns+Nr_i)/Nr_i`, so each collar is worth
1.7797, 1.3217, 1.2447, 1.1901 Nm at the output per Nm at the collar — and
**that trade runs the wrong way round.** The small row's collar is worth the
most, 1.78 against 1.19, while the equator row is the only one with the
diameter to carry a large machine: 121 mm of pitch radius against 29.5. Worth
knowing before anyone sizes a motor.

Willis constrains speeds only — nothing in it balances torque — and with
three or seven planets per row the mesh load share is set by manufacture and
deflection, not by kinematics. **The collars are the control surface: drag
current sets the torque equilibrium.** The only natural levelling in this
machine is electrical, between supercapacitor banks. It is a reconfigurable
fixed-ratio stack plus a power-split differential with an exact parameterised
ratio versus auxiliary speed. It is not a CVT, and the contract says so in
those words.

### The EM collar's numbers

24 slots, 28 poles, three phase. `q = S/(3P) = 2/7`; the balance gate
`S/(3 gcd(S,P)) = 2` is an integer; winding factor `kw = kp·kd = 0.933013`.
Cogging order `lcm(24,28) = 168` per revolution, torque-ripple order 84 —
exactly half, so the two are **not** independent, which the file states
rather than glossing. Against the mesh orders: rows 1 and 2 share no factor
with 168; row 3 shares 4; the equator row and the sun share 2.

Airgap 1 mm, active annulus r = 177…203 mm. At an **assumed** mean airgap
shear of 12 kPa — the low end of the 10–30 kPa band, flagged as an
assumption everywhere it propagates — `T = sigma·2pi(Rao³−Rai³)/3 = 70.88
Nm`, 2.23 kW at 300 rpm. The capacitor bank is 12 F over a 24 V…12 V rail
band: 2592 J usable of 3456 J, nine 2.7 V cells in series, 1.16 s of braking
at full assumed shear.

---

## 7. What is not modelled, and what is not started

**The cap-to-slice attachment is the important omission**, for the reason
section 6 gives: it is what decides whether the stack is a 89.47:1 series
reduction or a four-speed differential. Drawing it is the next piece of
work.

Also not modelled, and listed in `stack.scad`'s own report: the polar shaft
and floating spline sleeve that would make the series reading real — the
cavity for it is the cap's and the cap prints the wall it has to live in;
the bolts, at the pole joints and at both collars; the balls of a pole
joint, because `polar_cap.scad` publishes the ball radius but neither the
count nor the groove arc radius, so the member that forms the joint is
exactly the one that cannot place them with a computed clearance; the
housing and the stator's ground path; the chain and its pinion, and the
band's spur pinion; the EM collar's phase leads, terminals and bus; and the
brake collar the design table's third collar implies.

**Not started:** only the equator row has a band, because
`equator_band.scad` *is* the equator row's ring. Rows 1, 2 and 3 have no
register on their rings, so only an equator-row station can take a collar at
all — which is why the default configuration uses two equator-row stations
to show two collars. Three more bands is the next piece of work, and until
it exists the selection table of section 6 has one controllable row.

Four more gaps are in what a part *publishes* rather than in what it draws,
and `stack.scad` names each where it bites: the cap publishes its ball
radius but not the ball count or the groove arc radius; `pc_if_crown()`
publishes the crown's outer cone distance but not the inner end of its face,
so a caller can compute the coupling's flank clearance as an angle but its
arc in mm only at the outer end; neither collar publishes its torque *duty*
as a function, so the stack can compute what each clamp holds from published
geometry and declared friction but cannot compare it against what it must
hold — and that comparison is the one that decides whether the register
needs a key; and the chain collar's 22 mm bore overhangs the 20 mm
land-groove-land register at each end onto the relief, which the band
publishes and the collar publishes but nothing subtracted until now.

Also open:

- The retainer on a k = 3 row is a very large spherical shell, because the
  planets are three times the sun and their pins are correspondingly wide
  cones. It is geometrically consistent and structurally unexamined.
- `stack.scad` trips the export-time union budget on the band's group
  (393,204 triangles), so the export cannot confirm *that* pair is apart.
  The printed clearances are the statement. Everything else in the model is
  under the budget.
- No stress or deflection analysis anywhere. Gate 7 bounds the polar crown
  by a Lewis calculation and the EM torque figure rests on a declared shear
  assumption; nothing else is load-checked.

---

## 8. Reproducing all of it

```sh
cd scadforge/examples/transmission
cargo run --release -p scadforge -- -o /tmp/stack.stl  stack.scad
cargo run --release -p scadforge -- -o /tmp/stack.echo stack.scad   # the report
cargo run --release -p scadforge -- -D SL_I=3 -o /tmp/eq.stl slice.scad
```

`stack.scad`'s configuration is the `STK_ST` station table, a list of
`[row index into sg_rows(), collar code]` with collar code 0 bare, 1 chain,
2 EM. `slice.scad` takes `SL_I` = 0…3.

Never export to a path without an extension: the CLI rejects the format and
exits *before* evaluating, so the model never runs and the timing is
meaningless. That cost a day once and is recorded in
`GEOMETRY_DECISIONS.md`'s appendix.
