/*
  Admin dashboard behaviour.

  Plain DOM, no framework and no third-party script: this box may be firewalled
  down to the CDN's published ranges, so anything this page needs has to arrive
  from this page.

  One rule runs through the whole file — every mutation is followed by a refetch
  of /overview rather than a patch of the local copy. The guest list is dozens
  of rows, the round trip is imperceptible, and it makes it impossible for a
  number in a stat tile to disagree with the table underneath it.
*/
(() => {
  'use strict';

  const BASE = '/api/admin';

  // ── Tiny DOM helpers ──────────────────────────────────────────────────────

  const $ = (id) => document.getElementById(id);
  const $$ = (sel, root = document) => Array.from(root.querySelectorAll(sel));

  /** Builds an element. Text always goes in as text, never as markup — guest
   *  names and notes are typed by guests. */
  function el(tag, props = {}, children = []) {
    const node = document.createElement(tag);
    for (const [key, value] of Object.entries(props)) {
      if (value === undefined || value === null) continue;
      if (key === 'class') node.className = value;
      else if (key === 'text') node.textContent = value;
      else if (key === 'html') node.innerHTML = value;
      else if (key.startsWith('on')) node.addEventListener(key.slice(2), value);
      else if (key === 'dataset') Object.assign(node.dataset, value);
      else if (key in node) node[key] = value;
      else node.setAttribute(key, value);
    }
    for (const child of [].concat(children)) {
      if (child === null || child === undefined || child === false) continue;
      node.append(child);
    }
    return node;
  }

  const clear = (node) => { while (node.firstChild) node.firstChild.remove(); };

  /** "1 guest", "3 guests". Counts are read at a glance here, and a stat that
   *  says "1 invitations" makes the reader check the number twice. */
  const plural = (n, one, many) => `${n} ${n === 1 ? one : many || one + 's'}`;

  /** SQLite writes `datetime('now')`, which is UTC with a space in the middle
   *  and no zone marker. Left alone, browsers read it as local time and every
   *  reply drifts by the timezone offset. */
  function when(stamp) {
    if (!stamp) return '—';
    const parsed = new Date(stamp.replace(' ', 'T') + 'Z');
    if (Number.isNaN(parsed.getTime())) return stamp;
    return parsed.toLocaleString(undefined, {
      day: 'numeric', month: 'short', hour: '2-digit', minute: '2-digit',
    });
  }

  // ── Server ────────────────────────────────────────────────────────────────

  class ApiError extends Error {
    constructor(message, status) { super(message); this.status = status; }
  }

  /**
   * `own401` is for the sign-in POST alone. Everywhere else a 401 means the
   * session went away underneath us and the right response is to throw the
   * screen away and ask again; on the sign-in card it means the password was
   * wrong, and bouncing through showSignIn there would wipe the very message
   * that explains why.
   */
  async function call(path, { method = 'GET', body, own401 = false } = {}) {
    const response = await fetch(BASE + path, {
      method,
      credentials: 'same-origin',
      headers: body === undefined ? {} : { 'Content-Type': 'application/json' },
      body: body === undefined ? undefined : JSON.stringify(body),
    });

    if (response.status === 401 && !own401) {
      // The session went away underneath us — a restart, or twelve hours.
      showSignIn('Your session has expired. Please sign in again.');
      throw new ApiError('signed out', 401);
    }

    let payload = null;
    try { payload = await response.json(); } catch { /* empty body is fine */ }

    if (!response.ok) {
      throw new ApiError(payload?.error || 'Something went wrong.', response.status);
    }
    return payload;
  }

  // ── State ─────────────────────────────────────────────────────────────────

  const state = {
    parties: [],
    view: 'overview',
    partyFilter: 'all',
    partySearch: '',
    guestFilter: 'all',
    guestSearch: '',
    openParty: null,
  };

  const partyById = (id) => state.parties.find((p) => p.id === id) || null;

  /** How far along one invitation is. `partial` is the interesting state: a
   *  family where somebody answered and somebody did not. */
  function partyStatus(party) {
    const answered = party.members.filter((m) => m.attending !== null).length;
    if (answered === 0) return 'none';
    if (answered === party.members.length) return 'complete';
    return 'partial';
  }

  function counts() {
    const totals = {
      parties: state.parties.length,
      guests: 0, yes: 0, no: 0, waiting: 0,
      complete: 0, partial: 0, none: 0,
      plusOnesAllowed: 0, plusOnesNamed: 0,
    };

    for (const party of state.parties) {
      totals.guests += party.members.length;
      totals[partyStatus(party)] += 1;
      for (const member of party.members) {
        if (member.attending === true) totals.yes += 1;
        else if (member.attending === false) totals.no += 1;
        else totals.waiting += 1;
        // An allowance is an invitation to bring somebody; a name is somebody
        // actually coming. Only the second is a head at a table.
        if (member.plus_one) totals.plusOnesAllowed += 1;
        if (member.plus_one_name) totals.plusOnesNamed += 1;
      }
    }
    return totals;
  }

  /** How many named guests one invitation is bringing. */
  const plusOnesComing = (party) => party.members.filter((m) => m.plus_one_name).length;

  /** How a guest reads where their invitation carries a plus one: their name,
   *  and whoever they are bringing once the reply has named them. */
  function guestLabel(member) {
    if (!member.plus_one) return member.name;
    return `${member.name} + ${member.plus_one_name || 'guest'}`;
  }

  // ── Toasts ────────────────────────────────────────────────────────────────

  function toast(message, kind) {
    const node = el('div', { class: 'toast' + (kind === 'error' ? ' toast--error' : ''), text: message });
    $('toasts').append(node);
    setTimeout(() => node.remove(), kind === 'error' ? 6000 : 3000);
  }

  /** Every write goes through here, so a refused edit always says why in the
   *  same place, and always leaves the screen showing what the server has. */
  async function mutate(description, work) {
    try {
      await work();
      await refresh();
      toast(description);
      return true;
    } catch (e) {
      if (e.status !== 401) toast(e.message, 'error');
      return false;
    }
  }

  // ── Sign in ───────────────────────────────────────────────────────────────

  function showSignIn(message) {
    $('app').hidden = true;
    closeDrawer();
    closeModal();
    $('signin').hidden = false;
    const error = $('signin-error');
    error.textContent = message || '';
    error.hidden = !message;
    $('password').value = '';
    $('password').focus();
  }

  function showApp() {
    $('signin').hidden = true;
    $('app').hidden = false;
  }

  $('signin-form').addEventListener('submit', async (event) => {
    event.preventDefault();
    const button = $('signin-submit');
    const error = $('signin-error');
    button.disabled = true;

    try {
      await call('/session', {
        method: 'POST',
        own401: true,
        body: { password: $('password').value },
      });
      error.hidden = true;
      showApp();
      await refresh();
    } catch (e) {
      error.textContent = e.message;
      error.hidden = false;
      $('password').select();
    } finally {
      button.disabled = false;
    }
  });

  $('signout').addEventListener('click', async () => {
    try { await call('/session', { method: 'DELETE' }); } catch { /* already gone */ }
    state.parties = [];
    showSignIn();
  });

  // ── Navigation ────────────────────────────────────────────────────────────

  const VIEWS = {
    overview: { title: 'Overview', sub: 'Where the guest list stands.' },
    invitations: { title: 'Invitations', sub: 'One row per invitation — open one to record replies.' },
    guests: { title: 'Guests', sub: 'Every named guest, and the answer on file.' },
  };

  function setView(name) {
    state.view = name;
    for (const button of $$('.nav__item')) {
      button.classList.toggle('is-active', button.dataset.view === name);
    }
    for (const section of $$('.view')) {
      section.hidden = section.dataset.view !== name;
    }
    $('view-title').textContent = VIEWS[name].title;
    $('view-sub').textContent = VIEWS[name].sub;
  }

  $('nav').addEventListener('click', (event) => {
    const button = event.target.closest('.nav__item');
    if (button) setView(button.dataset.view);
  });

  $('refresh').addEventListener('click', () => refresh().then(() => toast('Refreshed')));

  // ── Loading ───────────────────────────────────────────────────────────────

  async function refresh() {
    const data = await call('/overview');
    state.parties = data.parties;
    render();
  }

  function render() {
    renderOverview();
    renderInvitations();
    renderGuests();
    if (state.openParty !== null) renderDrawer();
  }

  // ── Overview ──────────────────────────────────────────────────────────────

  function statTile(label, value, hint) {
    return el('div', { class: 'stat' }, [
      el('div', { class: 'stat__label', text: label }),
      el('div', { class: 'stat__value', text: String(value) }),
      hint ? el('div', { class: 'stat__hint', text: hint }) : null,
    ]);
  }

  function renderOverview() {
    const totals = counts();
    const stats = $('stats');
    clear(stats);

    stats.append(
      statTile('Invitations', totals.parties, `${totals.complete} fully replied`),
      statTile('Guests invited', totals.guests, `${plural(totals.parties, 'invitation')} sent`),
      // Heads at tables, named guests and their plus ones together — the number
      // the caterer asks for. The hint splits it, because the two halves are
      // chased differently: one by asking, the other by waiting.
      statTile('Attending', totals.yes + totals.plusOnesNamed,
        totals.plusOnesAllowed
          ? `${totals.yes} named · ${plural(totals.plusOnesNamed, 'guest')}`
          : null),
      statTile('Declined', totals.no),
      statTile('Awaiting reply', totals.waiting,
        `${plural(totals.none, 'invitation')} not started`),
    );

    // Only where somebody was offered a guest — on a list without plus ones the
    // tile would read 0 of 0 and say nothing.
    if (totals.plusOnesAllowed > 0) {
      stats.append(statTile('Plus ones named', totals.plusOnesNamed,
        `of ${totals.plusOnesAllowed} invited with a guest`));
    }

    // ── Progress bar
    const bar = $('progress');
    clear(bar);
    const share = (n) => (totals.guests ? (n / totals.guests) * 100 : 0);
    for (const [kind, n] of [['yes', totals.yes], ['no', totals.no], ['wait', totals.waiting]]) {
      if (n > 0) bar.append(el('div', { class: `progress__part progress__part--${kind}`, style: `width:${share(n)}%` }));
    }

    const legend = $('progress-legend');
    clear(legend);
    const swatches = [
      ['Attending', totals.yes, 'var(--yes)'],
      ['Declined', totals.no, 'var(--no)'],
      ['Awaiting', totals.waiting, '#d8dde4'],
    ];
    for (const [label, n, colour] of swatches) {
      const pct = totals.guests ? Math.round((n / totals.guests) * 100) : 0;
      legend.append(el('li', {}, [
        el('span', { class: 'dot', style: `background:${colour}` }),
        el('span', { text: label }),
        el('b', { text: `${n}` }),
        el('span', { text: `(${pct}%)` }),
      ]));
    }

    // Said outright, because the Attending tile above counts heads and this bar
    // counts answers: a plus one is a head with nobody's reply behind them.
    if (totals.plusOnesAllowed > 0) {
      legend.append(el('li', { class: 'hint',
        text: 'Shares count named guests only — a plus one has no reply of their own.' }));
    }

    // ── Latest replies
    const latest = $('latest');
    clear(latest);
    const replied = state.parties
      .filter((p) => p.last_reply_at)
      .sort((a, b) => (a.last_reply_at < b.last_reply_at ? 1 : -1))
      .slice(0, 8);

    if (replied.length === 0) {
      latest.append(el('tr', {}, [el('td', { class: 'empty', colSpan: 3, text: 'No replies yet.' })]));
    }
    for (const party of replied) {
      latest.append(el('tr', {}, [
        el('td', {}, [el('button', { class: 'row-link', text: party.name, onclick: () => openDrawer(party.id) })]),
        el('td', {}, [replyTally(party)]),
        el('td', { class: 'num', text: when(party.last_reply_at) }),
      ]));
    }

    // ── Notes
    const notes = $('notes');
    clear(notes);
    const withNotes = state.parties.filter((p) => p.note);
    if (withNotes.length === 0) {
      notes.append(el('li', { class: 'empty', text: 'No notes from guests yet.' }));
    }
    for (const party of withNotes) {
      notes.append(el('li', {}, [
        el('div', { class: 'notes__who', text: party.name }),
        el('div', { class: 'notes__text', text: party.note }),
      ]));
    }
  }

  /** The compact "2 yes · 1 no · 1 waiting" summary of one invitation.
   *
   *  A named plus one gets its own pill rather than being folded into the
   *  attending count: they are a head at a table, but nobody replied for them
   *  and no filter here answers for them. */
  function replyTally(party) {
    const yes = party.members.filter((m) => m.attending === true).length;
    const no = party.members.filter((m) => m.attending === false).length;
    const waiting = party.members.filter((m) => m.attending === null).length;
    const guests = plusOnesComing(party);

    const wrap = el('div', { class: 'tally' });
    if (yes) wrap.append(el('span', { class: 'pill pill--yes', text: `${yes} attending` }));
    if (guests) {
      wrap.append(el('span', { class: 'pill pill--yes', text: `+${plural(guests, 'guest')}` }));
    }
    if (no) wrap.append(el('span', { class: 'pill pill--no', text: `${no} declined` }));
    if (waiting) wrap.append(el('span', { class: 'pill pill--wait', text: `${waiting} awaiting` }));
    if (!wrap.children.length) wrap.append(el('span', { class: 'pill pill--none', text: 'nobody listed' }));
    return wrap;
  }

  // ── Invitations ───────────────────────────────────────────────────────────

  function matchesSearch(party, term) {
    if (!term) return true;
    const haystack = [party.name, party.note || '',
      ...party.members.map(guestLabel)].join(' ').toLowerCase();
    return haystack.includes(term);
  }

  function renderInvitations() {
    const body = $('party-rows');
    clear(body);

    const term = state.partySearch.trim().toLowerCase();
    const rows = state.parties.filter((party) =>
      (state.partyFilter === 'all' || partyStatus(party) === state.partyFilter) &&
      matchesSearch(party, term));

    $('party-count').textContent =
      `${rows.length} of ${plural(state.parties.length, 'invitation')}`;
    $('party-empty').hidden = rows.length > 0;

    for (const party of rows) {
      const note = party.note
        ? el('span', { class: 'truncate', text: party.note, title: party.note })
        : el('span', { class: 'truncate', text: '—' });

      body.append(el('tr', {}, [
        el('td', {}, [
          el('button', { class: 'row-link', text: party.name, onclick: () => openDrawer(party.id) }),
          el('span', { class: 'cell-sub', text: party.members.map(guestLabel).join(', ') }),
        ]),
        // "3 + 1" where a guest is coming: the second number is somebody to
        // seat and feed who is on no line of the guest list.
        el('td', {
          class: 'num',
          text: plusOnesComing(party)
            ? `${party.members.length} + ${plusOnesComing(party)}`
            : String(party.members.length),
        }),
        el('td', {}, [replyTally(party)]),
        el('td', {}, [note]),
        el('td', { class: 'num', text: when(party.last_reply_at) }),
        el('td', { class: 'actions' }, [
          el('button', { class: 'btn btn--sm', text: 'Open', onclick: () => openDrawer(party.id) }),
        ]),
      ]));
    }
  }

  $('party-search').addEventListener('input', (e) => {
    state.partySearch = e.target.value;
    renderInvitations();
  });

  $('party-filter').addEventListener('click', (event) => {
    const button = event.target.closest('.segmented__item');
    if (!button) return;
    state.partyFilter = button.dataset.filter;
    for (const item of $$('.segmented__item', $('party-filter'))) {
      item.classList.toggle('is-active', item === button);
    }
    renderInvitations();
  });

  $('new-invitation').addEventListener('click', openModal);

  // ── Guests ────────────────────────────────────────────────────────────────

  function guestRows() {
    const term = state.guestSearch.trim().toLowerCase();
    const rows = [];

    for (const party of state.parties) {
      for (const member of party.members) {
        const status = member.attending === true ? 'yes'
          : member.attending === false ? 'no' : 'awaiting';
        if (state.guestFilter !== 'all' && state.guestFilter !== status) continue;
        if (term && !`${guestLabel(member)} ${party.name}`.toLowerCase().includes(term)) continue;
        rows.push({ party, member, status });
      }
    }

    rows.sort((a, b) => a.member.name.localeCompare(b.member.name));
    return rows;
  }

  function renderGuests() {
    const body = $('guest-rows');
    clear(body);

    const rows = guestRows();
    const total = state.parties.reduce((n, p) => n + p.members.length, 0);
    $('guest-count').textContent = `${rows.length} of ${plural(total, 'guest')}`;
    $('guest-empty').hidden = rows.length > 0;

    for (const { party, member, status } of rows) {
      const pill = status === 'yes' ? el('span', { class: 'pill pill--yes', text: 'Attending' })
        : status === 'no' ? el('span', { class: 'pill pill--no', text: 'Declined' })
        : el('span', { class: 'pill pill--wait', text: 'Awaiting' });

      body.append(el('tr', {}, [
        el('td', {}, [
          el('span', { class: 'member__name', text: member.name }),
          member.plus_one
            ? el('span', {
              class: 'cell-sub',
              text: member.plus_one_name
                ? `bringing ${member.plus_one_name}`
                : 'invited with a guest, not named yet',
            })
            : null,
        ]),
        el('td', {}, [el('button', { class: 'row-link', text: party.name, onclick: () => openDrawer(party.id) })]),
        el('td', {}, [pill]),
        el('td', { class: 'num', text: when(member.replied_at) }),
        el('td', { class: 'actions' }, [replyChoice(member)]),
      ]));
    }
  }

  $('guest-search').addEventListener('input', (e) => {
    state.guestSearch = e.target.value;
    renderGuests();
  });

  $('guest-filter').addEventListener('click', (event) => {
    const button = event.target.closest('.segmented__item');
    if (!button) return;
    state.guestFilter = button.dataset.filter;
    for (const item of $$('.segmented__item', $('guest-filter'))) {
      item.classList.toggle('is-active', item === button);
    }
    renderGuests();
  });

  /** Attending / Declined / Clear.
   *
   *  "Clear" is not a third answer — it deletes the reply, putting the guest
   *  back to never having answered, which is what the schema means by the
   *  absence of a row and what the RSVP card shows them. */
  function replyChoice(member) {
    const current = member.attending === true ? 'yes' : member.attending === false ? 'no' : 'clear';

    const make = (value, label) => el('button', {
      class: 'choice__btn' + (current === value ? ' is-on' : ''),
      type: 'button',
      text: label,
      dataset: { value },
      onclick: () => {
        if (current === value) return;
        const attending = value === 'yes' ? true : value === 'no' ? false : null;
        mutate(
          attending === null ? `Cleared ${member.name}'s reply` : `Saved ${member.name}'s reply`,
          () => call(`/invitees/${member.id}/rsvp`, { method: 'PUT', body: { attending } }),
        );
      },
    });

    return el('div', { class: 'choice' }, [
      make('yes', 'Attending'), make('no', 'Declined'), make('clear', 'Clear'),
    ]);
  }

  // ── Drawer: one invitation in full ────────────────────────────────────────

  function openDrawer(partyId) {
    state.openParty = partyId;
    $('scrim').hidden = false;
    $('drawer').hidden = false;
    renderDrawer();
  }

  function closeDrawer() {
    state.openParty = null;
    $('scrim').hidden = true;
    $('drawer').hidden = true;
  }

  $('drawer-close').addEventListener('click', closeDrawer);
  $('scrim').addEventListener('click', closeDrawer);
  document.addEventListener('keydown', (event) => {
    if (event.key !== 'Escape') return;
    if (!$('modal').hidden) closeModal();
    else if (!$('drawer').hidden) closeDrawer();
  });

  function renderDrawer() {
    const party = partyById(state.openParty);
    // The invitation can disappear underneath the drawer — its last guest
    // removed, say — in which case there is nothing left to show.
    if (!party) { closeDrawer(); return; }

    $('drawer-title').textContent = party.name;
    const guests = plusOnesComing(party);
    $('drawer-sub').textContent = `${plural(party.members.length, 'guest')} · ` +
      (guests ? `bringing ${plural(guests, 'guest')} · ` : '') +
      (party.last_reply_at ? `last reply ${when(party.last_reply_at)}` : 'no reply yet');

    const body = $('drawer-body');
    clear(body);
    body.append(membersSection(party), noteSection(party), dangerSection(party));
  }

  function membersSection(party) {
    const list = el('div');

    for (const member of party.members) {
      const name = el('span', { class: 'member__name', text: member.name });

      const meta = el('span', {
        class: 'member__meta',
        text: member.replied_at ? `replied ${when(member.replied_at)}` : 'no reply on file',
      });

      // An allowance and a named guest are different states, and the difference
      // is exactly what the couple is waiting on, so the line says which it is.
      const guest = member.plus_one
        ? el('span', {
          class: 'member__meta',
          text: member.plus_one_name
            ? `bringing ${member.plus_one_name}`
            : 'invited with a guest, not named yet',
        })
        : null;

      const left = el('div', {}, [name, meta, guest]);

      const rename = el('button', {
        class: 'btn btn--sm', type: 'button', text: 'Rename',
        onclick: () => startRename(left, member),
      });

      const remove = el('button', {
        class: 'btn btn--sm btn--danger', type: 'button', text: 'Remove',
        onclick: () => {
          const last = party.members.length === 1;
          const warning = last
            ? `Remove ${member.name}? They are the only guest on this invitation, so the invitation goes too.`
            : `Remove ${member.name} from this invitation? Their reply is removed with them.`;
          if (!confirm(warning)) return;
          mutate(`Removed ${member.name}`, () => call(`/invitees/${member.id}`, { method: 'DELETE' }));
        },
      });

      list.append(el('div', { class: 'member' }, [
        left,
        el('div', { class: 'stack' }, [
          replyChoice(member),
          el('div', { class: 'inline-form' }, [
            rename, remove, plusOneToggle(member), moveControl(party, member),
          ]),
          member.plus_one ? plusOneForm(member) : null,
        ]),
      ]));
    }

    // Adding a guest to an existing invitation, which is the other half of
    // what the seeding script does with a CSV line.
    const input = el('input', {
      class: 'input', type: 'text',
      placeholder: 'Add a guest — end with +1 for a plus one',
    });
    const add = el('button', {
      class: 'btn', type: 'button', text: 'Add',
      onclick: async () => {
        const name = input.value.trim();
        if (!name) return;
        const ok = await mutate(`Added ${name}`, () =>
          call(`/parties/${party.id}/invitees`, { method: 'POST', body: { name } }));
        if (ok) input.value = '';
      },
    });
    input.addEventListener('keydown', (e) => { if (e.key === 'Enter') add.click(); });

    return el('section', {}, [
      el('h3', { class: 'section__title', text: 'Guests and replies' }),
      list,
      el('div', { class: 'inline-form' }, [input, add]),
    ]);
  }

  /** Gives a guest a plus one, or takes it away.
   *
   *  Taking it away deletes whoever was named under it — nothing on the card
   *  could clear a guest who is no longer offered one — so it asks first when
   *  there is a name to lose. */
  function plusOneToggle(member) {
    const on = member.plus_one;

    return el('button', {
      class: 'btn btn--sm', type: 'button', text: on ? 'Remove +1' : 'Allow +1',
      onclick: () => {
        if (on && member.plus_one_name && !confirm(
          `Remove ${member.name}'s plus one? ${member.plus_one_name} is named as their guest and goes too.`)) return;

        mutate(
          on ? `${member.name} no longer has a plus one` : `${member.name} may bring a guest`,
          () => call(`/invitees/${member.id}`, { method: 'PUT', body: { plus_one: !on } }),
        );
      },
    });
  }

  /** Who a guest with a plus one is bringing, recorded on their behalf.
   *
   *  Emptying the box withdraws the guest, which is the same thing an emptied
   *  box does on the RSVP card and in the note below. */
  function plusOneForm(member) {
    const input = el('input', {
      class: 'input', type: 'text',
      value: member.plus_one_name || '',
      placeholder: "Their guest's name — empty to withdraw",
    });

    const save = el('button', {
      class: 'btn btn--sm', type: 'button', text: 'Save guest',
      onclick: () => {
        const name = input.value.trim();
        if (name === (member.plus_one_name || '')) return;
        mutate(
          name ? `Guest saved for ${member.name}` : `Guest cleared for ${member.name}`,
          () => call(`/invitees/${member.id}/plus-one`, {
            method: 'PUT', body: { name: name || null },
          }),
        );
      },
    });

    input.addEventListener('keydown', (e) => { if (e.key === 'Enter') save.click(); });

    return el('div', { class: 'inline-form' }, [input, save]);
  }

  /** Swaps a guest's name for an input, in place. */
  function startRename(container, member) {
    clear(container);
    const input = el('input', { class: 'input', type: 'text', value: member.name });
    const save = el('button', {
      class: 'btn btn--sm btn--primary', type: 'button', text: 'Save',
      onclick: () => {
        const name = input.value.trim();
        if (!name || name === member.name) { renderDrawer(); return; }
        mutate(`Renamed to ${name}`, () => call(`/invitees/${member.id}`, { method: 'PUT', body: { name } }));
      },
    });
    const cancel = el('button', { class: 'btn btn--sm', type: 'button', text: 'Cancel', onclick: renderDrawer });

    input.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') save.click();
      if (e.key === 'Escape') renderDrawer();
    });

    container.append(el('div', { class: 'inline-form' }, [input, save, cancel]));
    input.focus();
    input.select();
  }

  /** Moving a guest between invitations changes who answers for whom, so it is
   *  a deliberate pick from a list rather than a drag. */
  function moveControl(party, member) {
    const select = el('select', { class: 'select btn--sm' });
    select.append(el('option', { value: '', text: 'Move to…' }));

    for (const other of state.parties) {
      if (other.id === party.id) continue;
      select.append(el('option', { value: String(other.id), text: other.name }));
    }

    select.addEventListener('change', () => {
      const target = select.value;
      select.value = '';
      if (!target) return;
      const name = partyById(Number(target))?.name || 'that invitation';
      if (!confirm(`Move ${member.name} to ${name}? They will then be asked about with that party.`)) return;
      mutate(`Moved ${member.name}`, () =>
        call(`/invitees/${member.id}`, { method: 'PUT', body: { party_id: Number(target) } }));
    });

    return select;
  }

  function noteSection(party) {
    const box = el('textarea', { class: 'textarea', rows: 4, value: party.note || '' });

    const save = el('button', {
      class: 'btn btn--primary btn--sm', type: 'button', text: 'Save note',
      onclick: () => {
        const note = box.value.trim();
        mutate(note ? 'Note saved' : 'Note cleared',
          () => call(`/parties/${party.id}/note`, { method: 'PUT', body: { note: note || null } }));
      },
    });

    return el('section', {}, [
      el('h3', { class: 'section__title', text: 'Note' }),
      box,
      el('p', { class: 'hint', text: party.note_updated_at ? `Last changed ${when(party.note_updated_at)}. Emptying the box clears it.` : 'Emptying the box clears it.' }),
      el('div', { class: 'inline-form' }, [save]),
    ]);
  }

  function dangerSection(party) {
    return el('section', { class: 'danger-zone' }, [
      el('h3', { class: 'section__title', text: 'Remove invitation' }),
      el('p', { class: 'hint', text: 'Deletes the invitation, everyone on it, their replies and the note. There is no undo.' }),
      el('div', { class: 'inline-form' }, [
        el('button', {
          class: 'btn btn--danger', type: 'button', text: 'Delete invitation',
          onclick: () => {
            if (!confirm(`Delete the invitation for ${party.name}? Everyone on it and every reply they gave goes with it.`)) return;
            mutate('Invitation deleted', async () => {
              await call(`/parties/${party.id}`, { method: 'DELETE' });
              closeDrawer();
            });
          },
        }),
      ]),
    ]);
  }

  // ── Modal: a new invitation ───────────────────────────────────────────────

  function openModal() {
    $('modal').hidden = false;
    $('modal-names').value = '';
    $('modal-error').hidden = true;
    $('modal-names').focus();
  }

  function closeModal() { $('modal').hidden = true; }

  $('modal-cancel').addEventListener('click', closeModal);
  $('modal').addEventListener('mousedown', (event) => {
    if (event.target === $('modal')) closeModal();
  });

  $('modal-form').addEventListener('submit', async (event) => {
    event.preventDefault();
    const names = $('modal-names').value.split('\n').map((n) => n.trim()).filter(Boolean);
    const error = $('modal-error');

    if (names.length === 0) {
      error.textContent = 'Add at least one guest.';
      error.hidden = false;
      return;
    }

    $('modal-submit').disabled = true;
    try {
      await call('/parties', { method: 'POST', body: { names } });
      closeModal();
      await refresh();
      toast(`Invitation created for ${names.join(' & ')}`);
    } catch (e) {
      if (e.status !== 401) {
        error.textContent = e.message;
        error.hidden = false;
      }
    } finally {
      $('modal-submit').disabled = false;
    }
  });

  // ── Boot ──────────────────────────────────────────────────────────────────

  setView('overview');

  if (document.body.dataset.signedIn === 'true') {
    showApp();
    refresh().catch((e) => { if (e.status !== 401) toast(e.message, 'error'); });
  } else {
    showSignIn();
  }
})();
