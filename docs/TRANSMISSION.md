# The spherical planetary stack: what it is, and what building it found

Seven files in `scadforge/examples/transmission/` describe one machine: a
four-row spherical-bevel planetary stack on a single polar axis, with a
swappable collar on the equatorial row that is either a chain sprocket or a
regenerative axial-flux motor.

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
| `stack.scad` | the machine | 1,624,244 | 69 | 0 | 0 | 0 |

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
answers it by stacking, biggest at the bottom:

```
station 0 is the equator row, apex at the sphere centre;
station n+1 sits a declared gap above station n.                     (16)
```

Every member of a row is a spherical shell sector, so its z extreme is at a
corner of its (cone distance, colatitude) rectangle, and (16) makes
consecutive rows' z intervals disjoint — separated by the planes
z = 121.991, 205.605, 273.739, each gap 2 mm. **Two solids on opposite sides
of a plane cannot touch, whatever their radii**, so the whole
between-station clearance argument is one comparison per station and it is
exact. The same sun test at the built stations returns 0 points inside on
all six pairs.

The four suns land at z = 118.794, 202.462, 269.936, 293.418, all at pitch
radius 23 mm. One shaft carries all four — which is what the contract means
by one count, one pitch diameter, stepped cones.

### The price

A machine that fitted inside the fundamental sphere would be 242 mm tall.
This one is **425.7 mm**, a factor of 1.76. A stack of four bevel sets is
long, and that length is the cost of the exactness, not a modelling
artefact. It is stated in the file rather than buried.

---

## 5. What assembling it found

Four defects, none visible from inside a part file.

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

One pattern in all four, and it is the lesson: *a number that two files have
to agree on was published by one and copied by the other, and the copy was
never compared with the original.* An assembly is where the comparison
happens. Until there was an assembly, every one of these was invisible, and
three of them had been sitting in files that validate clean.

---

## 6. The drive

The sun is common to every row. Each row's ring is a band with a register,
and a collar clamps that register: a chain sprocket, an EM rotor, or a brake.
The carriers are tied to one output — *that tie member is not modelled.*

With the carriers tied and one ring held, Willis gives
`w_sun / w_carrier = (Ns + Nr_i) / Ns` exactly:

| row | ratio | reduced | value |
|---|---|---|---|
| row 1 | 105/46 | 105/46 | 2.28261 |
| row 2 | 189/46 | 189/46 | 4.10870 |
| row 3 | 234/46 | 117/23 | 5.08696 |
| equator | 288/46 | 144/23 | 6.26087 |

**A four-speed**, spread 2.74286, in steps of 1.800, 1.238, 1.231. The steps
are uneven and the file says so: the counts were chosen to make each row
close exactly, not to space the ratios.

Holding the sun instead gives `w_ring/w_carrier = (Ns + Nr)/Nr` =
1.7797, 1.3217, 1.2447, 1.1901; holding the carrier gives
`w_ring/w_sun = -Ns/Nr`.

Two rings driven at once is the differential mode. Willis constrains speeds
only — nothing in it balances torque — and with three or seven planets per
row the mesh load share is set by manufacture and deflection, not by
kinematics. **The collars are the control surface: drag current sets the
torque equilibrium.** The only natural levelling in this machine is
electrical, between supercapacitor banks. This is a reconfigurable
fixed-ratio stack plus a power-split differential with an exact
parameterised ratio versus auxiliary speed. It is not a CVT, and the
contract says so in those words.

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

Not modelled, and listed in `stack.scad`'s own report: the polar shaft that
ties the four suns and the two cap splines together; the carrier tie that
makes the four speeds one output; the bolts; the housing and the stator's
ground path; the bearings other than the ball channel a cap *pair* forms;
the chain and its pinion; the EM collar's cable, terminals and bus.

**Not started:** rows 1, 2 and 3 have no register on their rings, so only
the equator row can take a collar at all. Three more bands is the next piece
of work, and until it exists the four-speed of section 6 has one controllable
row, not four.

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

`stack.scad` takes `STK_COLLAR` = 0 (bare register), 1 (chain, the default)
or 2 (EM), and `STK_CAPS`. `slice.scad` takes `SL_I` = 0…3.

Never export to a path without an extension: the CLI rejects the format and
exits *before* evaluating, so the model never runs and the timing is
meaningless. That cost a day once and is recorded in
`GEOMETRY_DECISIONS.md`'s appendix.
