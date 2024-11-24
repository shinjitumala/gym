import "typed-query-selector/strict"
import { StrictlyParseSelector } from "typed-query-selector/parser"
import * as p from "plotly.js"

const s = <S extends string>(s: S): StrictlyParseSelector<S> => {
    const a = document.querySelector(s)
    if (a === null) {
        throw `Not found '${s}'`
    }
    return a
}
const e = <K extends keyof HTMLElementTagNameMap>(k: K): HTMLElementTagNameMap[K] => {
    return document.createElement(k)
}

class MuscleGroup {
    name: string = ""
}

const color = (i: number) => {
    const c = [
        "red",
        "yellow",
        "blue",
        "green",
        "aqua",
        "fuchsia",
        "lime",
        "maroon",
        "navy",
        "olive",
        "purple",
        "silver",
        "gray",
        "teal",
        "black",
        "white",
    ]
    return c[i % c.length]
}

const load = async () => {
    const smgs = s("select#musclegroup");
    fetch("/mgs").then(e => e.json()).then(a => {
        const mgs: Array<MuscleGroup> = a;
        const x = new Array<Element>;
        for (const a of mgs) {
            const o = e("option");
            o.innerText = a.name
            o.value = a.name
            x.push(o)
        }

        smgs.replaceChildren(...x);
        smgs.selectedIndex = 0
        upd()
    })

    smgs.onchange = async () => {
        upd()
    }

    const range = s("select#range");
    range.onchange = async () => {
        upd()
    }
}

type Layout = Partial<p.Layout>;

const upd = async () => {
    const z = 0.02
    const d = 5;
    const w = 1 / d;
    const r = [...Array(d).keys()].map(e => [w * e + z, w * (e + 1) - z]).reverse()

    const mg = s("select#musclegroup").value;

    const t_weight = "weight kg"
    const c_weight = "red"
    const t_bf = "bodyfat %"
    const c_bf = "blue"
    const t_max = "theoretical max"
    const c_max = "black"

    var today = new Date(Date.now());
    var begin;
    const range = s("select#range");
    const t = new Date(today);
    switch (range.value) {
        case "week":
            t.setDate(t.getDate() - 7)
            begin = t
            break;
        case "2 weeks":
            t.setDate(t.getDate() - 14)
            begin = t
            break;
        case "3 weeks":
            t.setDate(t.getDate() - 21)
            begin = t
            break;
        case "4 weeks":
            t.setDate(t.getDate() - 28)
            begin = t
            break;
        case "month":
            t.setMonth(t.getMonth() - 1)
            begin = t
            break;
        case "year":
            t.setFullYear(t.getFullYear() - 1)
            begin = t
            break;
        case "all":
            begin = "2024-09-26" // hard-coded
            break;
    }

    const l: Layout = {
        xaxis: { title: "date", tickformat: "%Y-%m-%d %H:%M", range: [begin, today] },
        yaxis: { title: t_weight, domain: r[2], side: "left", color: c_weight },
        yaxis2: { title: t_bf, domain: r[2], side: "right", overlaying: "y", color: c_bf },
        yaxis3: { title: t_max, domain: r[0], color: c_max },
        yaxis4: { title: "calories kcal", domain: r[3], color: c_max, side: "left" },
        yaxis5: { title: "protein g", domain: r[4], color: c_max, side: "left" },
        yaxis6: { title: "sets", domain: r[1], side: "left" },
        barmode: "stack",
        hoverlabel: { namelength: -1 },
        height: 2400,
    }

    let weight = fetch("/weight").then(r => r.json()).then(j => {
        const weight = j as {
            date: Array<string>,
            kg: Array<number>,
            bodyfat: Array<number>,
            desc: Array<string>,
        }
        const t1: Partial<p.Data> = {
            x: weight.date,
            y: weight.kg,
            xaxis: "x",
            yaxis: "y",
            text: weight.desc,
            name: t_weight,
            line: {
                color: c_weight
            },
            mode: "lines+markers",
            showlegend: false,
        }

        const t2: Partial<p.Data> = {
            x: weight.date,
            y: weight.bodyfat,
            xaxis: "x",
            yaxis: "y2",
            text: weight.desc,
            name: t_bf,
            line: {
                color: c_bf
            },
            mode: "lines+markers",
            showlegend: false,
        }

        return [t1, t2]
    });

    let food = fetch("/food").then(r => r.json()).then(j => {
        const food = j as {
            string: {
                date: Array<string>,
                calories: Array<number>,
                protein: Array<number>,
                desc: Array<string>,
            }
        };

        const r: Partial<p.Data>[] = [];
        var idx = 0;
        for (const n in food) {
            const f = food[n as keyof typeof food]
            const t0: Partial<p.Data> = {
                x: f.date,
                y: f.calories,
                name: n,
                type: "bar",
                text: f.desc,
                xaxis: "x",
                yaxis: "y4",
                showlegend: false,
                marker: {
                    color: color(idx),
                }
            }

            const t1: Partial<p.Data> = {
                x: f.date,
                y: f.protein,
                name: n,
                type: "bar",
                text: f.desc,
                xaxis: "x",
                yaxis: "y5",
                showlegend: false,
                marker: {
                    color: color(idx),
                }
            }

            idx++;
            r.push(t0, t1)
        }
        return r
    });

    const mem = (await fetch("/map").then(j => j.json())) as {
        string: [string]
    };
    const es = (() => {
        const x = mem[mg as keyof typeof mem]
        if (x === undefined) {
            return []
        }
        return x
    })();

    const prog = fetch("/prog").then(j => j.json()).then(j => {
        const prog = j as {
            string: {
                date: Array<string>,
                max: Array<number>,
                desc: Array<string>,
            }
        }


        const r: Partial<p.Data>[] = [];
        var idx = 0;
        for (const e in prog) {
            if (!es.find(x => e.includes(x))) {
                continue
            }

            const s = prog[e as keyof typeof prog];

            const t2: Partial<p.Data> = {
                x: s.date,
                y: s.max,
                xaxis: "x",
                yaxis: "y3",
                text: s.desc,
                name: e,
                line: {
                    color: color(idx)
                },
                mode: "lines+markers",
                showlegend: false,
            }
            r.push(t2);
            idx++;
        }

        return r
    });

    const sets = fetch("/sets").then(j => j.json()).then(j => {
        const sets = j as {
            string: {
                string: {
                    date: Array<Date>,
                    place: Array<string>,
                    count: Array<number>,
                    desc: Array<string>,
                }
            }
        };

        const r: Partial<p.Data>[] = [];

        var idx = 0;
        const a = sets[mg as keyof typeof sets];
        for (const e in a) {
            const s = a[e as keyof typeof a];

            const t2: Partial<p.Data> = {
                x: s.date,
                y: s.count,
                xaxis: "x",
                yaxis: "y6",
                text: s.desc,
                name: e,
                type: "bar",
                showlegend: false,
            }
            r.push(t2);
            idx++;
        }

        return r
    });

    let a = await Promise.all([weight, food, prog, sets])

    p.newPlot("fig", a.flat(), <any>l)
}

const m = async () => {
    load();
}
window.onload = () => m();
