import { foldQuery } from './helpsearch';

/** La recherche de l'Aide, partagée entre la boîte de recherche et l'Aide (français et anglais) : le texte cherché et les rubriques qui le contiennent. */
class HelpSearch {
  query = $state('');
  /** Les rubriques où le texte a été trouvé et le nombre d'occurrences, calculés sur le texte affiché par `HelpSearchBox`. */
  matches = $state<string[]>([]);
  count = $state(0);

  /** Le texte cherché, plié ; vide : pas de recherche (moins de deux lettres). */
  get needle(): string {
    return foldQuery(this.query);
  }

  get active(): boolean {
    return this.needle !== '';
  }

  clear(): void {
    this.query = '';
  }
}

export const helpSearch = new HelpSearch();
