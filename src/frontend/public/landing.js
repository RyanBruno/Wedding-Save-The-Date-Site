// Scroll-driven save-the-date opening and gallery reveals.
(function () {
    var RETRY_MS = 50;
    var MAX_TRIES = 60;

    function boot(tries) {
        if (!window.gsap || !window.ScrollTrigger) {
            if (tries < MAX_TRIES) {
                window.setTimeout(function () { boot(tries + 1); }, RETRY_MS);
            }
            return;
        }

        var root = document.querySelector('.std-site');
        var opening = root && root.querySelector('.std-opening');
        var invite = opening && opening.querySelector('.std-invite');
        if (!root || !opening || !invite) return;

        var gsap = window.gsap;
        var ScrollTrigger = window.ScrollTrigger;
        gsap.registerPlugin(ScrollTrigger);
        ScrollTrigger.getAll().forEach(function (trigger) { trigger.kill(); });

        var left = invite.querySelector('.std-flap--left');
        var right = invite.querySelector('.std-flap--right');
        var top = invite.querySelector('.std-flap--top');
        var bottom = invite.querySelector('.std-flap--bottom');
        var cover = invite.querySelector('.std-cover-copy');
        var card = invite.querySelector('.std-card');
        var cardContent = card.querySelectorAll('.std-kicker, h1, .std-date, .std-place, .std-details, .std-gallery-link, .std-card__rule');

        if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
            gsap.set(left, { rotateY: -105, xPercent: -10 });
            gsap.set(right, { rotateY: 105, xPercent: 10 });
            gsap.set(top, { rotateX: 105, yPercent: -10 });
            gsap.set(bottom, { rotateX: -105, yPercent: 10 });
            gsap.set(cover, { autoAlpha: 0 });
            return;
        }

        gsap.set(card, { scale: .96 });
        gsap.set(cardContent, { autoAlpha: 0, y: 16 });

        var openingTimeline = gsap.timeline({
            defaults: { ease: 'none' },
            scrollTrigger: {
                trigger: opening,
                start: 'top top',
                end: 'bottom bottom',
                scrub: .7,
                invalidateOnRefresh: true,
            },
        });

        openingTimeline
            .to(cover, { autoAlpha: 0, y: -22, duration: .12 }, 0)
            .to(top, { rotateX: 112, yPercent: -8, duration: .55 }, .05)
            .to(bottom, { rotateX: -112, yPercent: 8, duration: .55 }, .05)
            .to(left, { rotateY: -112, xPercent: -8, duration: .55 }, .12)
            .to(right, { rotateY: 112, xPercent: 8, duration: .55 }, .12)
            .to(card, { scale: 1, duration: .28 }, .42)
            .to(cardContent, {
                autoAlpha: 1,
                y: 0,
                duration: .3,
                stagger: .025,
                ease: 'power2.out',
            }, .5);

        root.querySelectorAll('.std-gallery__item').forEach(function (item, index) {
            gsap.fromTo(item,
                { autoAlpha: 0, y: 32 },
                {
                    autoAlpha: 1,
                    y: 0,
                    duration: .8,
                    delay: (index % 3) * .04,
                    ease: 'power3.out',
                    scrollTrigger: {
                        trigger: item,
                        start: 'top 92%',
                    },
                }
            );
        });

        ScrollTrigger.refresh();
    }

    window.initLanding = function () { boot(0); };
})();
