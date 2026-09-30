import { Component, useEffect, type ReactNode } from "react";
import { Shell } from "./components/Shell";
import { Club } from "./pages/Club";
import { Comp, Nation } from "./pages/Comp";
import { Clubs, Comps, Events, Fixtures, History, Nations, Staff, Transfers } from "./pages/Directories";
import { Match } from "./pages/Match";
import { Inhabit } from "./pages/Inhabit";
import { Calendar } from "./pages/Calendar";
import { Contract } from "./pages/Contract";
import { Football } from "./pages/Football";
import { Journal } from "./pages/Journal";
import { Society } from "./pages/Society";
import { Life } from "./pages/Life";
import { Messages } from "./pages/Messages";
import { News } from "./pages/News";
import { PressFans } from "./pages/PressFans";
import { Relationships } from "./pages/Relationships";
import { Social } from "./pages/Social";
import { Portal } from "./pages/Portal";
import { Compare } from "./pages/Compare";
import { Bookmarks } from "./pages/Bookmarks";
import { Diagnostics } from "./pages/Diagnostics";
import { Ecosystem } from "./pages/Ecosystem";
import { Database } from "./pages/Database";
import { Help } from "./pages/Help";
import { Settings } from "./pages/Settings";
import { Overview } from "./pages/Overview";
import { People } from "./pages/People";
import { Person } from "./pages/Person";
import { Start } from "./pages/Start";
import { navigate, useRoute } from "./router";
import { refreshStatus, useStatus, useStatusLoaded } from "./store";
import { Button, ErrorState, Spinner } from "./ui/ui";

class Boundary extends Component<{ children: ReactNode; routeKey: string }, { error: Error | null; key: string }> {
  state = { error: null as Error | null, key: this.props.routeKey };
  static getDerivedStateFromError(error: Error) {
    return { error };
  }
  static getDerivedStateFromProps(props: { routeKey: string }, state: { key: string }) {
    return props.routeKey !== state.key ? { error: null, key: props.routeKey } : null;
  }
  render() {
    if (this.state.error) {
      return (
        <div className="page narrow">
          <ErrorState error={{ message: `This page failed to draw: ${this.state.error.message}` }} />
          <div><Button onClick={() => navigate("/overview")}>Go to the overview</Button></div>
        </div>
      );
    }
    return this.props.children;
  }
}

function Page() {
  const r = useRoute();
  switch (r.segs[0]) {
    case undefined:
      return <Home />;
    case "overview":
      return <Overview />;
    case "people":
      return <People />;
    case "person":
      return <Person />;
    case "today":
      return <Portal />;
    case "messages":
      return <Messages />;
    case "news":
      return <News />;
    case "calendar":
      return <Calendar />;
    case "football":
      return <Football />;
    case "contract":
      return <Contract />;
    case "life":
      return <Life />;
    case "relationships":
      return <Relationships />;
    case "press":
      return <PressFans />;
    case "social":
      return <Social />;
    case "society":
      return <Society />;
    case "journal":
      return <Journal />;
    case "me":
      return <Me />;
    case "compare":
      return <Compare />;
    case "bookmarks":
      return <Bookmarks />;
    case "settings":
      return <Settings />;
    case "help":
      return <Help />;
    case "diagnostics":
      return <Diagnostics />;
    case "development":
      return <Ecosystem />;
    case "database":
      return <Database />;
    case "inhabit":
      return <Inhabit />;
    case "match":
      return <Match />;
    case "club":
      return <Club />;
    case "comp":
      return <Comp />;
    case "nation":
      return <Nation />;
    case "clubs":
      return <Clubs />;
    case "comps":
      return <Comps />;
    case "nations":
      return <Nations />;
    case "fixtures":
      return <Fixtures />;
    case "transfers":
      return <Transfers />;
    case "events":
      return <Events />;
    case "history":
      return <History />;
    case "staff":
      return <Staff />;
    case "saves":
      return <Start inApp />;
    default:
      return (
        <div className="page narrow">
          <h1>That page does not exist</h1>
          <p className="muted">The address may be out of date.</p>
          <p><a href="#/overview">Go to the overview</a></p>
        </div>
      );
  }
}

function Me() {
  const st = useStatus();
  const id = st.perspective?.mode === "inhabit" ? st.perspective.person : null;
  useEffect(() => {
    navigate(id != null ? `/person/${id}` : "/people", { replace: true });
  }, [id]);
  return null;
}

function Home() {
  const st = useStatus();
  useEffect(() => {
    navigate(st.perspective?.mode === "inhabit" ? "/today" : "/overview", { replace: true });
  }, [st.perspective?.mode]);
  return null;
}

export function App() {
  const st = useStatus();
  const loaded = useStatusLoaded();
  const route = useRoute();
  useEffect(() => {
    void refreshStatus();
  }, []);

  if (!loaded) {
    return <div className="boot"><Spinner size={22} /></div>;
  }
  if (!st.open) {
    if (route.segs[0] === "database") return <div className="database-standalone"><Database /></div>;
    return <Start />;
  }
  return (
    <Shell>
      <Boundary routeKey={route.key}>
        <Page />
      </Boundary>
    </Shell>
  );
}
