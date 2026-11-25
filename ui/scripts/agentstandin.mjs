// The stand-in agent the Agents page's check (`agents.mjs`) runs: its package, which says what each turn
// took, and what it says of each size of work. Kept apart so that the check itself stays under the size ceiling.

export const talliesFor = (tells) => `extension: tallies
title: Tally
description: A stand-in that says what its turn took.
agent:
  program: ${tells}
  args:
    - ["tells"]
  input: \${ turn.prompt }
  output:
    - when: \${ event.kind == "end" }
      reply: \${ event.text }
      cost_usd: \${ event.cost }
      input_tokens: \${ event.in ?? 0 }
      output_tokens: \${ event.out ?? 0 }
      failed: \${ event.bad == true }
      error: \${ event.text }
`;

/** What the stand-in says of each size: what was sent to it, what it wrote, and what it cost. */
export const SIZES = { large: [120000, 9000, 0.41], middling: [40000, 2500, 0.09], small: [900, 100, 0.004] };
