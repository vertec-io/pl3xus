import Link from 'next/link';
import { Github, Book, Zap, Shield, Layers } from 'lucide-react';

export default function HomePage() {
  return (
    <main className="flex flex-col items-center justify-center min-h-[calc(100vh-4rem)] text-center px-4 overflow-hidden relative pb-20">
      <div className="absolute inset-0 -z-10 h-full w-full bg-[linear-gradient(to_right,#8080800a_1px,transparent_1px),linear-gradient(to_bottom,#8080800a_1px,transparent_1px)] bg-[size:14px_24px]"></div>

      <div className="absolute top-0 z-[-2] h-screen w-screen bg-background bg-[radial-gradient(ellipse_80%_80%_at_50%_-20%,rgba(234,179,8,0.3),rgba(255,255,255,0))]" />

      <h1 className="text-5xl font-extrabold tracking-tight sm:text-7xl mb-6 max-w-5xl mx-auto pt-20">
        Server-Authoritative<br />
        <span className="text-transparent bg-clip-text bg-gradient-to-r from-yellow-400 to-amber-600">
          Real-Time Sync
        </span>
      </h1>

      <p className="max-w-3xl mx-auto text-lg text-fd-muted-foreground mb-10 leading-relaxed">
        <strong>pl3xus</strong> is a Bevy ECS server + Leptos WASM client framework for building industrial-grade real-time applications. Zero boilerplate. Type-safe. Server-authoritative.
      </p>

      <div className="flex flex-col sm:flex-row gap-4 items-center justify-center mb-16">
        <Link href="/docs">
          <button className="inline-flex items-center justify-center h-12 px-8 text-sm font-semibold transition-all rounded-full bg-fd-primary text-fd-primary-foreground hover:bg-fd-primary/90 hover:scale-105 active:scale-95 shadow-lg shadow-amber-500/20">
            <Book className="w-4 h-4 mr-2" />
            Get Started
          </button>
        </Link>
        <Link href="https://github.com/vertec-io/pl3xus" target="_blank">
          <button className="inline-flex items-center justify-center h-12 px-8 text-sm font-semibold transition-all rounded-full border border-fd-border bg-fd-background hover:bg-fd-accent hover:text-fd-accent-foreground">
            <Github className="w-4 h-4 mr-2" />
            GitHub
          </button>
        </Link>
      </div>

      {/* Code Example */}
      <div className="w-full max-w-4xl mx-auto mb-24">
        <div className="bg-neutral-900 rounded-xl border border-fd-border overflow-hidden shadow-2xl">
          <div className="flex items-center gap-2 px-4 py-3 border-b border-fd-border bg-neutral-800/50">
            <div className="w-3 h-3 rounded-full bg-red-500"></div>
            <div className="w-3 h-3 rounded-full bg-yellow-500"></div>
            <div className="w-3 h-3 rounded-full bg-green-500"></div>
            <span className="ml-4 text-sm text-fd-muted-foreground font-mono">Show me the code</span>
          </div>
          <div className="p-6 text-left font-mono text-sm overflow-x-auto">
            <pre className="text-fd-muted-foreground">
              <code>{`// Server: Register component for sync
app.sync_component::<RobotPosition>(None);

// Client: Subscribe and display - that's it!
let positions = use_components::<RobotPosition>();

// Mutations with loading states (TanStack Query-inspired)
let mutation = use_mutation::<UpdatePosition>(|result| {
    match result {
        Ok(_) => log!("Updated!"),
        Err(e) => log!("Error: {e}"),
    }
});

mutation.send(UpdatePosition { x: 10.0, y: 20.0 });`}</code>
            </pre>
          </div>
        </div>
      </div>

      <div className="w-full max-w-6xl mx-auto space-y-24">
        {/* Feature Grid */}
        <div className="grid md:grid-cols-3 gap-8 text-left p-8 rounded-xl border border-fd-border/50 bg-fd-card/50 backdrop-blur-sm shadow-2xl relative overflow-hidden">
          <div className="absolute -top-12 -left-12 w-64 h-64 bg-amber-500/10 rounded-full blur-3xl -z-10"></div>
          <div className="absolute -bottom-12 -right-12 w-64 h-64 bg-amber-500/10 rounded-full blur-3xl -z-10"></div>

          <Feature
            icon={<Zap className="w-6 h-6 text-amber-500" />}
            title="Zero Boilerplate"
            desc="One line to register a component. One line to subscribe. Changes sync automatically."
          />
          <Feature
            icon={<Shield className="w-6 h-6 text-amber-500" />}
            title="Server-Authoritative"
            desc="The server is the single source of truth. Built-in authorization and entity control."
          />
          <Feature
            icon={<Layers className="w-6 h-6 text-amber-500" />}
            title="Industrial Grade"
            desc="Built for robotics, PLCs, and real-time control systems. Not just for games."
          />
        </div>

        {/* TanStack Query-Inspired API */}
        <div className="flex flex-col md:flex-row items-center gap-12 text-left">
          <div className="flex-1 space-y-6">
            <h2 className="text-3xl font-bold">TanStack Query-Inspired API</h2>
            <p className="text-fd-muted-foreground text-lg">
              Familiar patterns for developers coming from React. Mutations with loading states, queries with caching, and reactive updates that just work.
            </p>
            <ul className="space-y-2 text-fd-muted-foreground">
              <li className="flex items-center"><span className="w-2 h-2 bg-amber-500 rounded-full mr-3"></span><code className="text-sm">use_mutation</code> with loading/error states</li>
              <li className="flex items-center"><span className="w-2 h-2 bg-amber-500 rounded-full mr-3"></span><code className="text-sm">use_query</code> with caching</li>
              <li className="flex items-center"><span className="w-2 h-2 bg-amber-500 rounded-full mr-3"></span><code className="text-sm">use_mut_component</code> for direct edits</li>
            </ul>
          </div>
          <div className="flex-1 h-64 w-full bg-gradient-to-br from-neutral-900 to-neutral-800 rounded-xl border border-fd-border flex items-center justify-center">
            <span className="text-fd-muted font-mono">API Demo</span>
          </div>
        </div>

        {/* DevTools Section */}
        <div className="flex flex-col md:flex-row-reverse items-center gap-12 text-left">
          <div className="flex-1 space-y-6">
            <h2 className="text-3xl font-bold">Built-in DevTools</h2>
            <p className="text-fd-muted-foreground text-lg">
              Inspect entities, view component values, and edit state in real-time. Debug your application without leaving the browser.
            </p>
            <ul className="space-y-2 text-fd-muted-foreground">
              <li className="flex items-center"><span className="w-2 h-2 bg-amber-500 rounded-full mr-3"></span>Entity Browser</li>
              <li className="flex items-center"><span className="w-2 h-2 bg-amber-500 rounded-full mr-3"></span>Component Inspector</li>
              <li className="flex items-center"><span className="w-2 h-2 bg-amber-500 rounded-full mr-3"></span>Real-time Value Editing</li>
            </ul>
          </div>
          <div className="flex-1 h-64 w-full bg-gradient-to-bl from-neutral-900 to-neutral-800 rounded-xl border border-fd-border flex items-center justify-center">
            <span className="text-fd-muted font-mono">DevTools Preview</span>
          </div>
        </div>
      </div>
    </main>
  );
}

function Feature({ icon, title, desc }: { icon: React.ReactNode; title: string; desc: string }) {
  return (
    <div className="space-y-3">
      <div className="flex items-center gap-3">
        {icon}
        <h3 className="text-xl font-bold text-fd-foreground">{title}</h3>
      </div>
      <p className="text-sm text-fd-muted-foreground">{desc}</p>
    </div>
  )
}
