"""Assemble a1_proofs.tex from the refereed proofs note (../proofs/proofs.tex) and new text for this note.
Blocks taken from the proofs note are sliced by unique markers and relabelled; everything else is written here."""
import re
src = open('../proofs/proofs.tex').read()
def cut(a, b, inc_b=False):
    i = src.index(a); j = src.index(b, i)
    return src[i:j + (len(b) if inc_b else 0)]
def relabel(t):
    t = t.replace('T_U', '\\chi_U')
    t = re.sub(r'\bA([1-6])\b', r'P\1', t)
    t = re.sub(r'\bB1\b', 'K1', t)
    t = t.replace('\\cU', '\\mathcal{U}')
    return t

lem_forms = cut(r'In Lemma~\ref{lem:forms}, $u_1,\dots,u_N$ are independent', r'A bound of the form (e) for every moment order')
proof_V = cut(r'\paragraph{Proof of \eqref{eq:V}.}', r'\begin{lemma}[Moments of $\xi_U$]')
lem_xi_mixed = cut(r'\begin{lemma}[Moments of $\xi_U$]', r'Zero third moments enter through')
thm_proof = cut(r'\begin{proof}[Proof of Theorem~\ref{thm:main}]', r'\paragraph{Which moments are used where.}')
sim_proof = cut(r'\begin{proof}' + '\nBy construction,', r'So a fixed number of draws adds a relative mean-square error')
lem_kss = cut(r'\begin{lemma}[The KSS variance estimates]\label{lem:kss}', r'\begin{lemma}[Rate of the fitted coefficients]')

# ---- edits inside copied blocks -------------------------------------------------------------------
lem_forms = relabel(lem_forms)
proof_V = relabel(proof_V).replace(r'\paragraph{Proof of \eqref{eq:V}.}', r'\paragraph{Proof of the variance formula \eqref{eq:V}.}').replace(r'\eqref{eq:V}', r'\eqref{eq:V}')
lem_xi_mixed = relabel(lem_xi_mixed)
lem_xi_mixed = lem_xi_mixed.replace(r"By \eqref{eq:xi}, $1+\xi_U=u_U'\bar Q_Uu_U\ge0$", r"By \eqref{eq:xi}, $1+\xi_U=u_U'\bar Q_Uu_U\ge0$")
thm_proof = relabel(thm_proof)
for old, new in [
    (r'\begin{proof}[Proof of Theorem~\ref{thm:main}]', r'\begin{proof}[Proof of Propositions~\ref{prop:mean} and~\ref{prop:conc}]'),
    (r"All moments below are finite: by P4, P5 and Lemma~\ref{lem:forms} each piece of \eqref{eq:pieces} is", r"All moments below are finite: by P4, P5 and Lemma~\ref{lem:forms} each piece of \eqref{eq:pieces} is"),
    (r"$\E P_4=-\sum_U\tau_U^2\chi_U$. By \eqref{eq:TUid}, $4e_U-2\nu_Uk_{UU}-\tau_U^2\chi_U=0$ for every $U$, so",
     r"$\E P_4=-\sum_U\tau_U^2\chi_U$. By Lemma~\ref{lem:stayer}, $4e_U-2\nu_Uk_{UU}-\tau_U^2\chi_U=0$ for every $U$, so"),
    (r"this is $V+b(\Delta)$. This proves (i);" + "\n" + r"for $\Delta=0$ it is Proposition~2 of the memo with exact weights.",
     r"this is $V+b(\Delta)$, which is Proposition~\ref{prop:mean}."),
    (r"\medskip\noindent For (ii) we bound the variance of each piece and use",
     r"\medskip\noindent For Proposition~\ref{prop:conc} we bound the variance of each piece and use"),
    (r"so the three" + "\n" + r"sub-pieces $\xi_U\|g_U\|^2$, $2\xi_Ug_U'h_U$ and $\xi_U\|h_U\|^2$ of the outline are covered by",
     r"so"),
    (r"In terms of the outline's sub-pieces, $\xi_U\|g_U\|^2=\xi_U\|\bar g_U\|^2+Z_U^{1a}+Z_U^{1c}$," + "\n" + r"$2\xi_Ug_U'h_U=2\xi_U\bar g_U'h_U+Z_U^{1b}$, and $\xi_U\|h_U\|^2$ belongs to $Z_U^0$. $Z_U^0$" + "\n" + r"is a function of $(u_U,u_O)$ and $Z_{U'}^0$ of $(u_{U'},u_O)$.",
     r"$Z_U^0$ is a function of $(u_U,u_O)$ and $Z_{U'}^0$ of $(u_{U'},u_O)$."),
    (r"the second is at most" + "\n" + r"$\bnu^2k_{UU'}^2$. These are the two bounds printed in the outline. Summing over ordered pairs with",
     r"the second is at most" + "\n" + r"$\bnu^2k_{UU'}^2$. Summing over ordered pairs with"),
    (r"\medskip\noindent\emph{Step 5: piece (4).} The summands are independent, $1+\nu_U\ge1$, and by" + "\n" + r"Lemmas \ref{lem:TU} and \ref{lem:xi}, $\tau_U^2|\chi_U|\le4\bnu\,k_{UU}$ and $\E(1+\xi_U)^4\le8(1+c_Q)$. So",
     r"\medskip\noindent\emph{Step 5: piece (4).} The summands are independent, $1+\nu_U\ge1$, and by" + "\n" + r"Lemmas \ref{lem:stayer} and \ref{lem:xi}, $\tau_U^2|\chi_U|\le\bnu\,k_{UU}$ and $\E(1+\xi_U)^4\le8(1+c_Q)$. So"),
    (r"\medskip\noindent\emph{Step 6: assembly.} By \eqref{eq:four} and Steps 1 to 5 (recall $\bth\ge1$), the" + "\n" + r"first inequality of \eqref{eq:sharp} holds with",
     r"\medskip\noindent\emph{Step 6: assembly.} By \eqref{eq:four} and Steps 1 to 5 (recall $\bth\ge1$)," + "\n" + r"$\Var(\hat V)\le c_1\big(\|K\Theta a\|^2+\|K\Theta K\|^2\big)+c_2\,\bth^2\sum_U\rho_U^2$ with"),
    (r"For the second inequality, $\|K\Theta a\|\le\lambda\,\bth\,\|a\|$ and",
     r"Next, $\|K\Theta a\|\le\lambda\,\bth\,\|a\|$ and"),
    (r"$\sum_U\rho_U^2\le\max_U\rho_U\sum_U\rho_U\le\frac14\max_U\phi_U\,V^2$.",
     r"$\sum_U\rho_U^2\le\max_U\rho_U\sum_U\rho_U\le\frac14\max_U\phi_U\,V^2$. Hence" + "\n" + r"$\Var(\hat V)\le\bth^2\big(\frac12c_1\lambda^2V+\frac14c_2\max_U\phi_UV^2\big)$; dividing by $V^2$ gives" + "\n" + r"Proposition~\ref{prop:conc} with $c'=\max(\frac12c_1,\frac14c_2)$, because $\E\hat V=V+b(\Delta)$."),
]:
    assert thm_proof.count(old) == 1, old[:70]
    thm_proof = thm_proof.replace(old, new)
thm_proof, nB = re.subn(r"(?<![A-Za-z\\])B(?=(?:'|u_\{|B|\\,|=K|\)|\$|G_))", lambda m: '\\Lambda ', thm_proof)
thm_proof = thm_proof.replace('\\Lambda \'', '\\Lambda\'').replace('\\Lambda $', '\\Lambda$').replace('\\Lambda )', '\\Lambda)').replace('\\Lambda =', '\\Lambda=').replace('\\Lambda \\,', '\\Lambda\\,')
thm_proof = thm_proof.replace("\\Lambda B'", "\\Lambda\\Lambda'")
print('B -> Lambda replacements', nB, 'left:', re.findall(r".{12}(?<![A-Za-z\\])B(?![a-z]).{8}", thm_proof))
assert 'memo' not in thm_proof and 'outline' not in thm_proof, [m.start() for m in re.finditer('memo|outline', thm_proof)]
sim_proof = relabel(sim_proof).replace('z^{(d)}', '\\zeta^{(d)}')
assert sim_proof.count('This gives the equality and the last sentence.') == 1
sim_proof = sim_proof.replace('This gives the equality and the last sentence.', 'This gives the equality.')
lem_kss = relabel(lem_kss).replace(r'\label{lem:kss}', r'\label{lem:kssvar}')
for name, blk in (('forms', lem_forms), ('V', proof_V), ('xi', lem_xi_mixed), ('sim', sim_proof), ('kss', lem_kss)):
    assert 'memo' not in blk and 'outline' not in blk, name

A = r'''\section{Proofs}\label{app:proofs}

\subsection{Setting and notation}

The proofs use the following, more general, statement of the setting. Rows $i=1,\dots,n$ have outcomes
$y_i=\mu_i+\varepsilon_i$ with $\mu_i=x_i'\beta$; the regressors and $\beta$ are non-random. The estimator is
$\hat\theta=\sum_i\sum_\ell C_{i\ell}y_iy_\ell$, with $m=C\mu$, $\hat m=Cy$ and $V=\V[\hat\theta]$. $U$ denotes the
set of rows of a stayer, $\mathcal{U}$ the collection of these sets (one per stayer), $|U|\ge2$ the number of rows
of $U$ and $d_U=|U|-1$; $R$ is the set of movers' rows ($\cR$ in the text).

\begin{enumerate}[label=\textbf{(P\arabic*)},leftmargin=3.3em,itemsep=3pt]
\item \textbf{Independent errors.} $\varepsilon_1,\dots,\varepsilon_n$ are independent, with $\E\varepsilon_i=0$
and $0<\sigma_i^2=\E\varepsilon_i^2<\infty$.
\item \textbf{Exact weights.} $C$ is a non-random symmetric matrix with $C_{ii}=0$ for all $i$, and for every
$U\in\mathcal{U}$ there is a number $c_U$ with $C_{i\ell}=c_U$ for all $i\ne\ell$ in $U$. (Lemma~\ref{lem:stayer}:
the weights \eqref{eq:ustat} have this property.)
\item \textbf{Stayers.} The sets $U\in\mathcal{U}$ are pairwise disjoint and are determined by the regressors.
Within a set the means are equal, $\mu_i=\mu_U$, and so are the variances, $\sigma_i^2=\tau_U$. With
$\bar y_U$ the mean of the outcomes of $U$,
\[
q_U=\frac{1}{d_U}\sum_{i\in U}(y_i-\bar y_U)^2,
\]
and every row of $U$ receives $\st_i^2=q_U$. ($q_U$ is $q_g$ of \eqref{eq:q} and $\tau_U$ is $\sigma_g^2$.)
\item \textbf{Stayers' errors.} For every row $i$ of a set, $\E\varepsilon_i^3=0$,
$\E\varepsilon_i^4=\kappa\,\sigma_i^4$ and $\E\varepsilon_i^8\le\bar c\,\sigma_i^8$. The text takes $\kappa=3$;
the proofs allow any $\kappa\ge1$.
\item \textbf{Movers' rows.} For $i\in R$, $\E\varepsilon_i^4\le\bar\kappa\,\sigma_i^4$ for a constant
$\bar\kappa\ge\kappa$. These rows receive $\st_i^2=\sigma_i^2+\Delta_i$ with non-random numbers $\Delta_i$.
\item $V>0$.
\end{enumerate}
P1 to P3 are A1 and A2 of the text, P4 is A3 with the moment bound of Proposition~\ref{prop:conc}, and nothing
is asked of the third moments or the kurtosis of movers' errors.

\paragraph{The estimator.} For a stated kurtosis $\kappa$ let
\begin{equation}\label{eq:nuh}
\nu_U=\frac{2}{|U|-1}+\frac{\kappa-3}{|U|},\qquad
\chi_U=c_U^2\big[4\,(2-|U|)+2\,(\kappa-3)\,(|U|-1)\big],\qquad \bnu=2+|\kappa-3| ,
\end{equation}
and
\begin{equation}\label{eq:VhatA}
\hat V=4\sum_i\hat m_i^{\,2}\,\st_i^2-2\sum_i\sum_\ell C_{i\ell}^2\,\st_i^2\,\st_\ell^2
-\sum_{U\in\mathcal{U}}\chi_U\,\frac{q_U^2}{1+\nu_U}.
\end{equation}
For $\kappa=3$, $\chi_U/(1+\nu_U)=-4c_U^2(|U|-2)(|U|-1)/(|U|+1)$, and \eqref{eq:VhatA} is
$\hat\V[\hat\theta]$ of \eqref{eq:Vhat}. $b(\Delta)$ is defined in \eqref{eq:b}.

\paragraph{Standardised notation.} Let $u_i=\varepsilon_i/\sigma_i$, $a_i=\sigma_im_i$ and
$K_{i\ell}=\sigma_iC_{i\ell}\sigma_\ell$, so that $K$ is symmetric with zero diagonal, and let $w=a+Ku$, so that
$\sigma_i\hat m_i=w_i$. For index sets $I,J$, $K_{IJ}$ is the block of $K$ with rows in $I$ and columns in $J$,
$K_{I\cdot}$ the rows in $I$, and $u_I$, $a_I$, $w_I$ the subvectors. $\|\cdot\|$ is the Euclidean norm of a vector
and the Frobenius norm of a matrix; $\|\cdot\|_\op$ is the operator norm. We use $\|XY\|\le\|X\|_\op\|Y\|$,
$\|X\|_\op\le\|X\|$ and $|\tr(XY)|\le\|X\|\,\|Y\|$ freely.

For $U\in\mathcal{U}$ let $\Pi_U=I-\mathbf{1}\mathbf{1}'/|U|$, the projection that removes the mean of the set, and
$\bar Q_U=\Pi_U/d_U$. Since the means are equal within $U$ and $\varepsilon_i=\sqrt{\tau_U}\,u_i$ on $U$,
\begin{equation}\label{eq:xi}
\xi_U:=\frac{q_U}{\tau_U}-1=u_U'\,\bar Q_U\,u_U-1,\qquad \st_i^2=\sigma_i^2\,(1+\xi_U)\ \ (i\in U).
\end{equation}
$\xi_U$ depends on $u_U$ only, so the $\xi_U$ are independent across sets, and $\xi_U$ is independent of the errors
outside $U$. Put $\xi_i=\xi_U$ for $i\in U$ and $\xi_i=0$ for $i\in R$, and let
\begin{equation}\label{eq:G}
G_U=2\,\bar Q_U+(\kappa-3)\diag\big(\bar Q_U\big)=2\,\bar Q_U+\frac{\kappa-3}{|U|}\,I .
\end{equation}
Note that $\tr(\bar Q_U^2)=1/d_U$ and that the diagonal entries of $\bar Q_U$ equal $1/|U|$, so that
$\nu_U=2\tr(\bar Q_U^2)+(\kappa-3)\sum_{j\in U}\bar Q_{U,jj}^{\,2}$.

For the movers' rows let $\vartheta_i=1+\Delta_i/\sigma_i^2$ ($i\in R$), $\vartheta_i=1$ for rows in sets,
$\Theta=\diag(\vartheta_i)$ and $\bth=\max(1,\max_{i\in R}|\vartheta_i|)$. Then
$\st_i^2=\sigma_i^2\,\vartheta_i\,(1+\xi_i)$ for every row, and \eqref{eq:VhatA} reads
\begin{equation}\label{eq:pieces}
\begin{aligned}
\hat V&=P_1+P_2+P_3+P_4, &
P_1&=4\,w'\Theta w, \qquad P_2=4\sum_{U\in\mathcal{U}}\xi_U\,\|w_U\|^2,\\
P_3&=-2\sum_i\sum_\ell K_{i\ell}^2\,\vartheta_i\vartheta_\ell\,(1+\xi_i)(1+\xi_\ell), &
P_4&=-\sum_{U\in\mathcal{U}}\tau_U^2\,\chi_U\,\frac{(1+\xi_U)^2}{1+\nu_U}.
\end{aligned}
\end{equation}
(For the first term, $\hat m_i^{\,2}\st_i^2=w_i^2\vartheta_i(1+\xi_i)$, and $\vartheta_i=1$ where $\xi_i\ne0$.)
By Lemma~\ref{lem:forms} below, $V=4\|a\|^2+2\|K\|^2$, which is \eqref{eq:V}. Let $k_{UU'}=\|K_{UU'}\|^2$,
$k_U=\|K_{U\cdot}\|^2=\sum_{i\in U}\sum_\ell K_{i\ell}^2$, $\rho_U=\|a_U\|^2+k_U$, and let
$\lambda=\lambda_{\max}=\|K\|_\op$. The share of Proposition~\ref{prop:conc} is $\phi_U=(4\|a_U\|^2+2k_U)/V$. Since
the sets are disjoint,
\begin{equation}\label{eq:facts}
2\rho_U\le\phi_UV,\qquad \sum_{U}\rho_U\le\|a\|^2+\|K\|^2\le\tfrac12V,\qquad
\sum_{U'\in\mathcal{U}}k_{UU'}\le k_U,\qquad k_{UU}\le k_U\le\rho_U .
\end{equation}
All results are bounds for one design with explicit constants; along a sequence of designs the constants of
the assumptions are the same for every design.

\subsection{Lemmas}

\begin{lemma}[A stayer's weights]\label{lem:stayer}
\textup{(a)} For the weights \eqref{eq:ustat} and two rows $i\ne\ell$ of stayer $g$: $P_{i\ell}=1/T_g$,
$B_{i\ell}=A_{gg}/T_g^2$ and $C_{i\ell}=A_{gg}/(T_g(T_g-1))$; moreover $m_i=m_\ell$.
\textup{(b)} Under P2, $K_{UU}=\tau_Uc_U(\mathbf{1}\mathbf{1}'-I)$, and with $e_U:=\tr(K_{UU}^2G_U)$,
\[
4\,e_U-2\,\nu_U\,k_{UU}=\tau_U^2\,\chi_U,\qquad \tau_U^2\,|\chi_U|\le\bnu\,k_{UU} .
\]
\end{lemma}

\begin{proof}
(a) The rows of stayer $g$ have the same regressor vector $x_i=x_g$, and no other row involves the person effect
of $g$. Let $\epsilon_g$ be the unit vector of that effect. Then
$S_{xx}\epsilon_g=\sum_jx_j(x_j'\epsilon_g)=T_gx_g$, so $S_{xx}^{-1}x_i=\epsilon_g/T_g$. Hence
$P_{i\ell}=x_i'\epsilon_g/T_g=1/T_g$ and $B_{i\ell}=\epsilon_g'A\epsilon_g/T_g^2=A_{gg}/T_g^2$, also for $i=\ell$.
With $M_{i\ell}=-1/T_g$ and $M_{ii}=1-1/T_g$, \eqref{eq:ustat} gives
$C_{i\ell}=\frac{A_{gg}}{T_g^2}\big(1+\frac{1}{T_g-1}\big)=\frac{A_{gg}}{T_g(T_g-1)}$. Finally
$m_i-m_\ell=\sum_{j\ne i}C_{ij}\mu_j-\sum_{j\ne\ell}C_{\ell j}\mu_j=C_{i\ell}(\mu_\ell-\mu_i)=0$, because the rows
$i$ and $\ell$ of $C$ agree outside the columns $i$ and $\ell$ and $\mu_i=\mu_\ell$.
(b) Write $t=|U|$ and $J=\mathbf{1}\mathbf{1}'$. The first claim is P2 with $\sigma_i^2=\tau_U$ on $U$. Then
$K_{UU}^2=\tau_U^2c_U^2\big((t-2)J+I\big)$, so $k_{UU}=\tr K_{UU}^2=\tau_U^2c_U^2\,t(t-1)$, and, since $J\Pi_U=0$,
$\tr(K_{UU}^2\bar Q_U)=\tau_U^2c_U^2\tr(\Pi_U)/d_U=\tau_U^2c_U^2$. By \eqref{eq:G} and \eqref{eq:nuh},
\[
4e_U-2\nu_Uk_{UU}=\tau_U^2c_U^2\Big[8+4(\kappa-3)(t-1)-4t-2(\kappa-3)(t-1)\Big]=\tau_U^2\chi_U .
\]
For the bound, $|2-t|\le t-1$ gives $|\chi_U|\le c_U^2(t-1)(4+2|\kappa-3|)=2\bnu\,c_U^2(t-1)$, and
$c_U^2(t-1)\tau_U^2=k_{UU}/t\le k_{UU}/2$.
\end{proof}

'''
A += lem_forms.rstrip() + '\n\n' + proof_V.rstrip() + '\n\n' + lem_xi_mixed.rstrip() + '\n\n'
A += r'''Zero third moments enter through $\E[\xi_Uu_U]=0$ only. The second part of \eqref{eq:condid} is the
conditional-expectation identity that removes the own-set and mixed terms below.

\subsection{Propositions~\ref{prop:mean} and~\ref{prop:conc}}

We prove both for any $\kappa\ge1$ and any non-random $\Delta_i$ on the movers' rows. Step~0 is
Proposition~\ref{prop:mean}; it uses P1 to P3, the third and fourth moments of stayers' errors in P4, and second
moments of movers' errors, that is, A1 to A3. Steps 1 to 6 are Proposition~\ref{prop:conc} and use the moment
bounds of P4 and P5; the constants $c_1,c_2$ depend only on $\bar c$, $\bar\kappa$ and $\kappa$.

'''
A += thm_proof.rstrip() + '\n\n'
A += r'''\paragraph{Departures from A2 and A3.} Three calculations give the mean of the estimator \eqref{eq:Vhat} of the
text (the adjustment for $\kappa=3$) when stayers' errors depart from A2 or A3 and movers' rows have their true
variances.

\emph{Another kurtosis.} If stayers' errors have kurtosis $\kappa\ne3$, Step~0 gives
$\E\hat V-V=\sum_U\tau_U^2\big[\chi_U(\kappa)-\chi_U(3)\,(1+\nu_U(\kappa))/(1+\nu_U(3))\big]$, where the arguments
indicate the kurtosis used in \eqref{eq:nuh}. Inserting \eqref{eq:nuh},
\begin{equation}\label{eq:kurt}
\E\hat V-V=2\,(\kappa-3)\sum_U\tau_U^2\,c_U^2\,\frac{(|U|-1)^2(|U|+4)}{|U|\,(|U|+1)} .
\end{equation}

\emph{Unequal variances within a stayer.} Let the rows of $U$ have normal errors with variances $v_1,\dots,v_t$,
$t=|U|$, with mean $\bar v$ and $s_U^2=\frac{1}{t-1}\sum_j(v_j-\bar v)^2$. The rows of $U$ have a common $m_i$ and
common weights to every row outside $U$, so every term of $V$ and of $\E\hat V$ that involves one row of $U$ at a
time depends on the $v_j$ through $\sum_jv_j$ only, and $\E q_U=\bar v$: these terms agree. The terms within $U$ do
not. With $S=\sum_j\varepsilon_j$ and $Q=\sum_j\varepsilon_j^2$ over $U$ we have $q_U=(Q-S^2/t)/(t-1)$ and
$\sum_{i\in U}(S-\varepsilon_i)^2=(t-2)S^2+Q$. The normal moments $\E S^4=3(\sum_jv_j)^2$ and
$\E Q^2=\E[S^2Q]=(\sum_jv_j)^2+2\sum_jv_j^2$ then give, after collecting terms,
\begin{equation}\label{eq:uneq}
\E\hat V-V=6\sum_Uc_U^2\,s_U^2\,\frac{(|U|-1)^2(|U|+4)}{|U|\,(|U|+1)}\;\ge0 .
\end{equation}

\emph{Skewness.} If P4's $\E\varepsilon_i^3=0$ is replaced by $\E\varepsilon_i^3=\mu_{3U}$ on $U$,
only the mean of $P_2$ changes: $\E[\xi_Uu_p]=\E u_p^3/|U|$ for $p\in U$, so
$\E[\xi_U\,2g_U'h_U]=2\,\E[g_U]'K_{UU}\E[\xi_Uu_U]=2\,a_U'K_{UU}\mathbf{1}\,\E u^3/|U|$ in the notation of
\eqref{eq:gh}. With $a_U=\tau_U^{1/2}m_U\mathbf{1}$, $K_{UU}\mathbf{1}=\tau_Uc_U(|U|-1)\mathbf{1}$ and
$\E u^3=\mu_{3U}/\tau_U^{3/2}$ this adds $8\,(|U|-1)\,c_U\,m_U\,\mu_{3U}$ to $\E\hat V$ for each stayer, as stated
after Proposition~\ref{prop:mean}.

'''
A += r'''\subsection{Proposition~\ref{prop:fitted} and Corollary~\ref{cor:transfer}}

Now the regression structure matters. $X$ is the $n\times k$ matrix with rows $x_i'$, of full column rank,
$P=XS_{xx}^{-1}X'$, $M=I-P$, and $\hat\sigma_i^2=y_i\,(My)_i/M_{ii}$. Assumption~1 of KSS is used in this form:
\begin{enumerate}[label=\textbf{(K\arabic*)},leftmargin=3.3em,itemsep=3pt]
\item for all rows, $\E\varepsilon_i^4\le\bar\mu_4$, $\sigma_i^2\ge\underline\sigma^2>0$, $P_{ii}\le\bar P<1$ and
$|x_i'\beta|\le\bar\mu$. Consequently $\sigma_i^2\le\bar\sigma^2:=\bar\mu_4^{1/2}$, $M_{ii}\ge1-\bar P$, and P5's
moment bound holds with $\bar\kappa=\bar\mu_4/\underline\sigma^4$.
\end{enumerate}

'''
A += lem_kss.rstrip() + '\n\n'
A += r'''\begin{proof}[Proof of Proposition~\ref{prop:fitted}]
Write $\hat V(v)$ for \eqref{eq:VhatA} when the movers' rows receive the values $v=(v_i)_{i\in R}$, and let
$v^*_i=\psi_i(\eta^*)$, $\hat v_i=\st_i^2$ the fitted values, $e_i=\hat v_i-v^*_i$ and $\delta=\hat\eta-\eta^*$. Let
$\Omega$ be the event that $\|\delta\|\le r$ and $\hat v_i=\psi_i(\hat\eta)$ for all $i\in R$. By assumption and
Markov's inequality, $\Pr(\Omega^c)\to0$ as $n_0\to\infty$.

\emph{Step A: an identity.} $\hat V(v)$ is a polynomial of degree two in $v$: the first term of
\eqref{eq:VhatA} is linear in the movers' values, the second is quadratic, and the third does not involve them.
Expanding around $v^*$, with $C$ symmetric,
\begin{equation}\label{eq:expand}
\hat V(\hat v)-\hat V(v^*)=4\sum_{i\in R}e_i\Big[\hat m_i^{\,2}-\sum_\ell C_{i\ell}^2\,\st_\ell^{*2}\Big]
-2\sum_{i\in R}\sum_{\ell\in R}C_{i\ell}^2\,e_ie_\ell ,
\end{equation}
where $\st_\ell^{*2}=q_U$ for $\ell\in U$ and $\st_\ell^{*2}=v^*_\ell$ for $\ell\in R$. This is an algebraic
identity, valid for every realisation, whatever the dependence between $\hat\eta$ and the other terms.

\emph{Step B: the brackets.} Let $J=\sum_{i\in R}\hat m_i^{\,2}+\sum_{i\in R}\sum_\ell C_{i\ell}^2|\st_\ell^{*2}|$.
Since $C_{i\ell}^2=K_{i\ell}^2/(\sigma_i^2\sigma_\ell^2)$ and $\sigma_i^2\ge\underline\sigma^2$, we have
$\sum_{i,\ell\in R}C_{i\ell}^2\le\underline\sigma^{-4}\|K\|^2\le\frac12\underline\sigma^{-4}V$, and
$J\le\underline\sigma^{-2}\big[\|w_R\|^2+\sum_{i\in R}\sum_\ell K_{i\ell}^2\theta^*_\ell\big]$ with
$\theta^*_\ell=|\st_\ell^{*2}|/\sigma_\ell^2$, which is $1+\xi_U$ on a set and at most
$\bth:=\max(1,\bar\psi/\underline\sigma^2)$ on $R$. By Lemma~\ref{lem:forms}(c),
$(\E\|w_R\|^4)^{1/2}\le2c_4^{1/2}(\|a_R\|^2+\|K_{R\cdot}\|^2)$; by Minkowski's inequality and
$\E(1+\xi_U)^2=1+\nu_U$, the second sum has $L^2$ norm at most $\max(\bth,(1+\bnu)^{1/2})\,\|K_{R\cdot}\|^2$. As
$\|a_R\|^2+\|K_{R\cdot}\|^2\le\frac12V$,
\[
\big(\E J^2\big)^{1/2}\le c_J\,V,\qquad c_J=\frac{1}{2\,\underline\sigma^2}\Big(2c_4^{1/2}+\bth+(1+\bnu)^{1/2}\Big).
\]
\emph{Step C: the fitted values.} On the event $\Omega$ we have $|e_i|\le L\|\delta\|$ for all
$i\in R$, so by \eqref{eq:expand}
$|\hat V(\hat v)-\hat V(v^*)|\le4L\|\delta\|\,J+2L^2\|\delta\|^2\sum_{i,\ell\in R}C_{i\ell}^2$. By the
Cauchy--Schwarz inequality and $\E\|\delta\|^2\le c_\eta/n_0$,
\[
\E\Big[\big|\hat V(\hat v)-\hat V(v^*)\big|\,;\,\Omega\Big]\le V\Big(4L\,c_J\,c_\eta^{1/2}\,n_0^{-1/2}+L^2\underline\sigma^{-4}\,c_\eta\,n_0^{-1}\Big),
\]
so that $[\hat V(\hat v)-\hat V(v^*)]/V=O_p(n_0^{-1/2})$, because $\Pr(\Omega^c)\to0$.

\emph{Step D: the estimator at $v^*$.} $\hat V(v^*)$ is the estimator of Propositions~\ref{prop:mean}
and~\ref{prop:conc} with the non-random $\Delta_i=v^*_i-\sigma_i^2$, for which $|\vartheta_i|\le\bth$. Hence
$\E\hat V(v^*)=V+b(\Delta)$ and
$\E|\hat V(v^*)-V-b(\Delta)|\le c'^{1/2}\,\bth\,(\max_U\phi_U+\lambda_{\max}^2/V)^{1/2}\,V$. Steps C and D and
Markov's inequality give the proposition.
\end{proof}

\begin{proof}[Proof of Corollary~\ref{cor:transfer}]
Let $\|b_i\|\le\bar b$ and $\|z_i\|\le\bar z$, let $\underline\omega>0$ bound the two smallest eigenvalues from
below, and write $G_z=n_{\cR}^{-1}\sum_{i\in\cR}z_iz_i'$, which is non-random. The statistic is
$\hat\eta=(\hat N_1,\hat N_2)$ with
\[
\hat N_1=\frac{1}{n_{\cR}}\sum_{i\in\cR}z_i\,\hat\sigma_i^2,\qquad
\hat N_2=\frac{1}{n_{\cR}}\sum_{i\in\cR}z_ib_i'\,\hat\sigma_i^2,
\]
a vector and a matrix of fixed dimensions, and $\eta^*=(N_1^*,N_2^*)$ is the same with $\sigma_i^2$ in place of
$\hat\sigma_i^2$.

\emph{Rate.} Lemma~\ref{lem:kssvar}, applied to each entry of $\hat N_1$ and $\hat N_2$ (with $t_i=z_{ij}/n_{\cR}$ or
$t_i=z_{ij}b_{ik}/n_{\cR}$ on $\cR$ and zero elsewhere) and summed, gives
$\E\|\hat N_1-N_1^*\|^2\le c_\sigma\bar z^2/n_{\cR}$ and $\E\|\hat N_2-N_2^*\|^2\le c_\sigma\bar z^2\bar b^2/n_{\cR}$. So
$\E\|\hat\eta-\eta^*\|^2\le c_\eta/n_0$ with $n_0=n_{\cR}$ and $c_\eta=c_\sigma\bar z^2(1+\bar b^2)$. The leave-out
estimates of different rows need not be independent for this.

\emph{The fitted values as a function of $\eta$.} Without floors, the two steps are the following maps of
$\eta=(N_1,N_2)$. Step~1: $c_z=G_z^{-1}N_1$ and $s_i=z_i'c_z$. Step~2: with $x_i=s_i\,b_i$, the coefficient is
$H(\eta)^{-1}\varrho(\eta)$, where
\[
H(\eta)=\frac{1}{n_{\cR}}\sum_{i\in\cR}x_ix_i',\qquad \varrho(\eta)=N_2'c_z ,
\]
because $n_{\cR}^{-1}\sum_is_ib_i\hat\sigma_i^2=\hat N_2'c_z$ at $N_2=\hat N_2$. The fitted value is
\[
\psi^\circ_i(\eta)=x_i(\eta)'\,H(\eta)^{-1}\varrho(\eta) .
\]
At $\eta^*$ it equals $v_i^*$: $H(\eta^*)^{-1}\varrho(\eta^*)$ is the coefficient $d^*$ of the regression of
$\sigma_i^2$ on $s_i^*b_i$. The maps $\eta\mapsto s_i,x_i,H,\varrho$ are polynomials in $(N_1,N_2)$ with bounded
coefficients, and the inverse is handled by $H^{-1}-H_*^{-1}=H^{-1}(H_*-H)H_*^{-1}$;
$\|G_z^{-1}\|_\op\le1/\underline\omega$, the smallest eigenvalue of $H(\eta^*)$ is at least $\underline\omega$, and
$\|N_1^*\|$, $\|N_2^*\|$ are bounded by $\bar z\bar\sigma^2$ and $\bar z\bar b\bar\sigma^2$. So there are $r_1>0$ and
$L$, depending only on these constants, such that for $\|\eta-\eta^*\|\le r_1$ the smallest eigenvalue of
$H(\eta)$ is at least $\underline\omega/2$ and the maps $\eta\mapsto s_i$ and $\eta\mapsto\psi^\circ_i(\eta)$ are
Lipschitz with constant $L$ for every $i$.

\emph{Floors.} The estimator floors $\hat s_i$ at $\hat f_s=0.05\,n_{\cR}^{-1}\sum_{i\in\cR}\hat\sigma_i^2$, which is
$0.05$ times the first entry of $\hat N_1$, and the fitted value at a random number $\hat f_v\ge0$. When a floor
binds for some row the fitted values are no longer the function $\psi^\circ$ of $\hat\eta$; this is why
Proposition~\ref{prop:fitted} asks for the representation $\st_i^2=\psi_i(\hat\eta)$ only with probability tending
to one. By assumption there is a margin $\iota>0$ with $s_i^*\ge f_s^*+\iota$ for all $i\in\cR$, where $f_s^*$ is
$0.05$ times the first entry of $N_1^*$, and $\Pr\big(\hat f_v\le\min_{i\in\cR}v_i^*-\iota\big)\to1$. Put
$\psi_i=\psi_i^\circ$ on $\{\|\eta-\eta^*\|\le r_1\}$ and $\psi_i=0$ elsewhere, and
$r=\min\big(r_1,\iota/(2L+2)\big)$. If $\|\hat\eta-\eta^*\|\le r$, then $\hat s_i-\hat f_s\ge\iota-(L+1)r>0$, so
the first floor binds for no row, and the fitted value before the last floor is
$\psi_i^\circ(\hat\eta)\ge v_i^*-\iota/2$; on the event in the last display it exceeds $\hat f_v$, so the last
floor does not bind either. Hence $\st_i^2=\psi_i(\hat\eta)$ for all $i\in\cR$ with probability tending to one,
and the conditions of Proposition~\ref{prop:fitted} hold, with $\bar\psi$ a bound on $|v_i^*|$ that follows from
the bounds above. Floors that keep binding in large samples are not covered. If $\sigma_i^2=s_i^*\,b_i'd$ on
$\cR$, the regression of $\sigma_i^2$ on $s_i^*b_i$ fits exactly, so $v_i^*=\sigma_i^2$ and $\Delta=0$. If
$\sigma_i^2=(z_i'c)(b_i'd)$ and $\sum_{i\in\cR}z_i\,(z_i'c)\,(b_i'd-\bar\pi)=0$ for a constant $\bar\pi\neq0$, then
$c_z^*=\bar\pi\,c$, so $s_i^*$ is proportional to $z_i'c$ and the first case applies.
\end{proof}

'''
A += r'''\subsection{Proposition~\ref{prop:sim}: the simulated double sum}

Let $\eta^{(1)},\dots,\eta^{(D_s)}$ be independent $N(0,I_n)$ vectors, drawn independently of the data, and
$\zeta^{(d)}_\ell=\st_\ell\,\eta^{(d)}_\ell$. $\hat V_{D_s}$ is \eqref{eq:VhatA} with the double sum replaced by
$\frac1{D_s}\sum_d\sum_i\st_i^2\big(\sum_\ell C_{i\ell}\zeta^{(d)}_\ell\big)^2$. We show, under P1 to P6 and
$0\le\vartheta_i\le\bth$,
\[
\E\big[(\hat V_{D_s}-\hat V)^2\big]=\frac{8}{D_s}\,\E\tr\big(\tilde K^4\big)\le\frac{c_3\,\bth^4}{D_s}\,\lambda_{\max}^2\,\|K\|^2,
\qquad \tilde K_{i\ell}=\st_iC_{i\ell}\st_\ell,\quad c_3=16(1+\bnu)+48(1+c_Q)+32\,\bnu^2 ,
\]
which gives the proposition with $c''=c_3/2$ because $\|K\|^2\le\frac12V$. In the proof $D$ stands for $D_s$.

'''
A += sim_proof.rstrip() + '\n\n'
A += r'''\subsection{A5 and the conditions of KSS}

KSS's Theorem~2 assumes their Assumption~1 and
(i) $V^{-1}\max_i\big((\tilde x_i'\beta)^2+(\check x_i'\beta)^2\big)\to0$,
(ii) $\lambda_1^2/\sum_\ell\lambda_\ell^2\to0$, where $\lambda_1,\dots,\lambda_r$ are the non-zero eigenvalues of
$\tilde A=S_{xx}^{-1/2}AS_{xx}^{-1/2}$, $|\lambda_1|$ the largest, $\tilde x_i=AS_{xx}^{-1}x_i$ and
$\check x_i=\sum_\ell M_{i\ell}\frac{B_{\ell\ell}}{1-P_{\ell\ell}}x_\ell$.

\begin{lemma}\label{lem:kss}
Let $C$ be the weights \eqref{eq:ustat}, and suppose $\underline\sigma^2\le\sigma_i^2\le\bar\sigma^2$ and
$P_{ii}\le\bar P<1$ for all $i$. Then
\[
\frac{\lambda_{\max}^2}{V}\le\frac{\bar\sigma^4}{2\,\underline\sigma^4(1-\bar P)^2}\cdot\frac{\lambda_1^2}{\sum_\ell\lambda_\ell^2},
\qquad
\max_U\phi_U\le\frac{\bar T}{V}\Big(8\,\bar\sigma^2\max_i\big[(\tilde x_i'\beta)^2+(\check x_i'\beta)^2\big]+2\lambda_{\max}^2\Big),
\]
where $\bar T$ is the largest number of rows of a stayer. So \textup{(ii)} implies condition \textup{(b)} of
Section~\ref{sec:results}, and, when $\bar T$ is bounded, \textup{(i)} and \textup{(ii)} imply condition \textup{(a)}.
\end{lemma}

\begin{proof}
Let $Z=XS_{xx}^{-1/2}$, so that $Z'Z=I$, $P=ZZ'$ and $B=Z\tilde AZ'$, where $B$ is the matrix with entries
$B_{i\ell}$. $B$ and $\tilde A$ have the same non-zero eigenvalues, so $\|B\|_\op=|\lambda_1|$ and
$\|B\|^2=\sum_\ell\lambda_\ell^2$. With $D_B=\diag(B_{ii}/M_{ii})$, $C=B-\frac12(MD_B+D_BM)$, and since $PM=0$
and $PBP=B$, $PCP=B$; a projection does not increase the Frobenius norm, so $\|C\|\ge\|B\|$. Also
$|B_{ii}|=|(Z\tilde AZ')_{ii}|\le|\lambda_1|\,(ZZ')_{ii}=|\lambda_1|\,P_{ii}$, so
$\|D_B\|_\op\le|\lambda_1|\bar P/(1-\bar P)$ and $\|C\|_\op\le\|B\|_\op+\|D_B\|_\op\le|\lambda_1|/(1-\bar P)$. Now
$K=\Sigma^{1/2}C\Sigma^{1/2}$ with $\Sigma=\diag(\sigma_i^2)$, so
$\lambda_{\max}\le\bar\sigma^2\|C\|_\op\le\bar\sigma^2|\lambda_1|/(1-\bar P)$ and
$V\ge2\|K\|^2\ge2\underline\sigma^4\|C\|^2\ge2\underline\sigma^4\sum_\ell\lambda_\ell^2$. This is the first
inequality. For the second, $M\mu=0$ gives $m=C\mu=B\mu-\frac12MD_B\mu$, and $B\mu=(\tilde x_i'\beta)_i$,
$MD_B\mu=(\check x_i'\beta)_i$, so $m_i^2\le2(\tilde x_i'\beta)^2+\frac12(\check x_i'\beta)^2$; moreover
$\sum_\ell K_{i\ell}^2=(K^2)_{ii}\le\lambda_{\max}^2$. Hence
$\phi_UV=\sum_{i\in U}\big(4\sigma_i^2m_i^2+2(K^2)_{ii}\big)$ is at most $|U|$ times the bracket.
\end{proof}

\subsection{Proposition~\ref{prop:q1} and Corollary~\ref{cor:q1}: one dominant direction}

\paragraph{Setting.} In addition to S1, S3, S4 and S5, let $\omega\in\mathbb{R}^n$ be non-random with
$\sum_i\omega_i^2=1$ (in the text $\omega_i=w_{i1}$; the symbol $w$ is in use), and let $C^{\rm r}$ be a matrix that
satisfies S2 (in the text, the weights of the remainder: they are formed as in \eqref{eq:ustat} from $A^{\rm r}$,
so Lemma~\ref{lem:stayer} applies to them). Put $\hat b=\omega'y$ and
$\hat\theta_{\rm r}=\sum_i\sum_\ell C^{\rm r}_{i\ell}y_iy_\ell$, and let $m^{\rm r}=C^{\rm r}\mu$,
$a^{\rm r}_i=\sigma_im_i^{\rm r}$, $K^{\rm r}_{i\ell}=\sigma_iC^{\rm r}_{i\ell}\sigma_\ell$,
$w^{\rm r}=a^{\rm r}+K^{\rm r}u$, $\lambda_{\rm r}=\|K^{\rm r}\|_\op$, and $k^{\rm r}_{UU}$, $k^{\rm r}_U$,
$\rho^{\rm r}_U$, $\phi^{\rm r}_U$ be the quantities of the first subsection formed with $C^{\rm r}$; in particular
$V_{\rm r}:=\V[\hat\theta_{\rm r}]=4\|a^{\rm r}\|^2+2\|K^{\rm r}\|^2>0$ and $2\rho^{\rm r}_U\le\phi^{\rm r}_UV_{\rm r}$.
Let $\varpi_i=\omega_i\sigma_i$.

\paragraph{The covariance matrix.} $\hat b-\E\hat b=\varpi'u$ and
$\hat\theta_{\rm r}-\E\hat\theta_{\rm r}=2a^{{\rm r}\prime}u+u'K^{\rm r}u$. $K^{\rm r}$ has zero diagonal, so by
Lemma~\ref{lem:forms}(b) $\E[(\varpi'u)(u'K^{\rm r}u)]=0$ with second moments only, and
\[
\Sigma_{bb}=\|\varpi\|^2=\sum_i\omega_i^2\sigma_i^2,\qquad
\Sigma_{b{\rm r}}=2\,\varpi'a^{\rm r}=2\sum_i\omega_im_i^{\rm r}\sigma_i^2,\qquad \Sigma_{\rm rr}=V_{\rm r},
\]
which is \eqref{eq:q1cov}; no third moment enters. The estimates are
$\hat\Sigma_{bb}^{\rm lo}=\sum_i\omega_i^2\hat\sigma_i^2$,
$\hat\Sigma_{b{\rm r}}=2\sum_i\omega_i\hat m_i^{\rm r}\st_i^2$ with $\hat m^{\rm r}=C^{\rm r}y$, and
$\hat\Sigma_{\rm rr}$, which is \eqref{eq:VhatA} formed with $C^{\rm r}$; $\hat\Sigma^{\rm lo}$ is the matrix with
these three entries. Let
$d_{\rm r}(\Delta)=2\sum_{i\in R}\omega_im_i^{\rm r}\Delta_i$, and let $b_{\rm r}(\Delta)$ be \eqref{eq:b} formed with
$C^{\rm r}$ and $m^{\rm r}$.

\begin{proof}[Proof of Proposition~\ref{prop:q1}]
\emph{(i) $\hat\Sigma_{bb}^{\rm lo}$.} Lemma~\ref{lem:kssvar} with $t_i=\omega_i^2$ gives
$\E\hat\Sigma_{bb}^{\rm lo}=\Sigma_{bb}$ and
$\Var(\hat\Sigma_{bb}^{\rm lo})\le c_\sigma\sum_i\omega_i^4\le c_\sigma\max_i\omega_i^2$. By K1,
$\Sigma_{bb}\ge\underline\sigma^2\sum_i\omega_i^2=\underline\sigma^2$. So
$\E[(\hat\Sigma_{bb}^{\rm lo}/\Sigma_{bb}-1)^2]\le c_b\max_i\omega_i^2$ with $c_b=c_\sigma/\underline\sigma^4$. No
model and no assumption on stayers is used.

\emph{(ii) $\hat\Sigma_{b{\rm r}}$: the mean.} Since $\sigma_i\hat m^{\rm r}_i=w^{\rm r}_i$ and
$\st_i^2=\sigma_i^2\vartheta_i(1+\xi_i)$, with $\vartheta_i=1$ where $\xi_i\ne0$,
\[
\hat\Sigma_{b{\rm r}}=Q_1+Q_2,\qquad Q_1=2\,\varpi'\Theta\,w^{\rm r},\qquad
Q_2=2\sum_{U\in\mathcal{U}}Y_U,\quad Y_U=\xi_U\,\varpi_U'w^{\rm r}_U .
\]
$\E Q_1=2\varpi'\Theta a^{\rm r}=2\sum_i\omega_im_i^{\rm r}\sigma_i^2\vartheta_i=\Sigma_{b{\rm r}}+d_{\rm r}(\Delta)$,
because $\sigma_i^2\vartheta_i=\sigma_i^2+\Delta_i$ on $R$. For $Q_2$ split $w^{\rm r}_U=g_U+h_U$ as in
\eqref{eq:gh}, with $h_U=K^{\rm r}_{UU}u_U$ and $g_U=a^{\rm r}_U+K^{\rm r}_{U,-U}u_{-U}$. Given $u_{-U}$, $g_U$ is
fixed, so by Lemmas~\ref{lem:xi} and~\ref{lem:mixed}
\begin{equation}\label{eq:Ycond}
\E\big[Y_U\,\big|\,u_{-U}\big]=\E[\xi_U]\,\varpi_U'g_U+\varpi_U'K^{\rm r}_{UU}\,\E[\xi_Uu_U]=0 .
\end{equation}
Hence $\E Q_2=0$ and $\E\hat\Sigma_{b{\rm r}}=\Sigma_{b{\rm r}}+d_{\rm r}(\Delta)$. The zero third moments of
stayers' errors are used here, through $\E[\xi_Uu_U]=0$; movers' third moments play no role, because their
rows receive non-random values. If the errors of stayer $g$ have a common third moment $\mu_{3g}$, then
$\E[\xi_Uu_j]=\E u_j^3/|U|$, and with Lemma~\ref{lem:stayer}(b) the mean of $\hat\Sigma_{b{\rm r}}$ shifts by
$2\sum_g(T_g-1)\,w_{g1}\,c_g^{\rm r}\,\mu_{3g}$.

\emph{(iii) $\hat\Sigma_{b{\rm r}}$: the variance.} $Q_1-\E Q_1=2\,(K^{\rm r}\Theta\varpi)'u$, so
$\Var(Q_1)=4\|K^{\rm r}\Theta\varpi\|^2\le4\,\bth^2\lambda_{\rm r}^2\,\Sigma_{bb}$. For $Q_2$,
$\Var(Q_2)=4\sum_U\E Y_U^2+4\sum_{U\ne U'}\E[Y_UY_{U'}]$.

\emph{Terms of the same set.} By the Cauchy--Schwarz inequality and $\|w^{\rm r}_U\|^2\le2\|g_U\|^2+2\|h_U\|^2$,
$Y_U^2\le2\,\|\varpi_U\|^2\,\xi_U^2\big(\|g_U\|^2+\|h_U\|^2\big)$. $g_U$ is independent of $\xi_U$, so
$\E[\xi_U^2\|g_U\|^2]=\nu_U\big(\|a^{\rm r}_U\|^2+\|K^{\rm r}_{U,-U}\|^2\big)$. And
$\|h_U\|^2=u_U'(K^{\rm r}_{UU})^2u_U$ with a positive semidefinite matrix of trace $k^{\rm r}_{UU}$, so by the
Cauchy--Schwarz inequality, Lemma~\ref{lem:xi} and Lemma~\ref{lem:forms}(e),
$\E[\xi_U^2\|h_U\|^2]\le(\E\xi_U^4)^{1/2}(\E\|h_U\|^4)^{1/2}\le(\E\xi_U^4)^{1/2}\big(\E(u_U'(K^{\rm r}_{UU})^2u_U)^4\big)^{1/4}
\le c_Q^{1/2}\big(8(c_Q+1)\big)^{1/4}k^{\rm r}_{UU}\le2\,(c_Q+1)\,k^{\rm r}_{UU}$. Since
$\|a^{\rm r}_U\|^2+\|K^{\rm r}_{U,-U}\|^2+k^{\rm r}_{UU}=\rho^{\rm r}_U$,
\[
\E Y_U^2\le c_Y\,\|\varpi_U\|^2\,\rho^{\rm r}_U,\qquad c_Y=2\,\bnu+4\,(c_Q+1).
\]
\emph{Terms of two different sets.} Fix $U\ne U'$, let $\Lambda=K^{\rm r}_{UU'}$ and let $O$ be the set of rows outside
$U\cup U'$; put $\bar g_U=a^{\rm r}_U+K^{\rm r}_{UO}u_O$, so that $w^{\rm r}_U=\bar g_U+h_U+\Lambda u_{U'}$, and
\[
Y_U=Y_U^0+Y_U^1,\qquad Y_U^0=\xi_U\,\varpi_U'(\bar g_U+h_U),\qquad Y_U^1=\xi_U\,\varpi_U'\Lambda\,u_{U'},
\]
and likewise for $U'$ with $\Lambda'$. $Y_U^0$ is a function of $(u_U,u_O)$ and $Y_{U'}^0$ of $(u_{U'},u_O)$. As in
\eqref{eq:Ycond}, $\E[Y_U^0\mid u_O,u_{U'}]=0$, so $\E[Y_U^0Y_{U'}^0]=0$. Next,
$Y_U^0\,Y_{U'}^1=\xi_{U'}\cdot\xi_U\,\varpi_U'(\bar g_U+h_U)\,(\Lambda\varpi_{U'})'u_U$; given $u_O$, the second
factor depends on $u_U$ only and the first on $u_{U'}$ only, the two blocks are independent, and $\E\xi_{U'}=0$: so
$\E[Y_U^0Y_{U'}^1]=0$, and by symmetry $\E[Y_U^1Y_{U'}^0]=0$. Finally
$Y_U^1Y_{U'}^1=\big(\xi_U\,u_U'\Lambda\varpi_{U'}\big)\big(\xi_{U'}\,u_{U'}'\Lambda'\varpi_U\big)$ is a product of a
function of $u_U$ and a function of $u_{U'}$, each with mean zero because $\E[\xi_Uu_U]=0$. All products are
integrable (polynomials of degree at most four in the errors of each of the two sets, and at most two in the
errors outside them). So
$\E[Y_UY_{U'}]=0$ for $U\ne U'$: the cross terms vanish exactly.

\emph{Assembly.} The sets are disjoint, so $\sum_U\|\varpi_U\|^2\le\Sigma_{bb}$, and
$\rho^{\rm r}_U\le\frac12\phi^{\rm r}_UV_{\rm r}$. Hence
\[
\Var(Q_2)=4\sum_U\E Y_U^2\le4\,c_Y\max_U\rho^{\rm r}_U\sum_U\|\varpi_U\|^2\le2\,c_Y\max_U\phi^{\rm r}_U\,\Sigma_{bb}\,\Sigma_{\rm rr},
\]
and with $\Var(Q_1+Q_2)\le2\Var Q_1+2\Var Q_2$ and $\bth\ge1$,
\[
\E\big[(\hat\Sigma_{b{\rm r}}-\Sigma_{b{\rm r}}-d_{\rm r}(\Delta))^2\big]\le
c_{b{\rm r}}\,\bth^2\Big(\max_U\phi^{\rm r}_U+\frac{\lambda_{\rm r}^2}{\Sigma_{\rm rr}}\Big)\Sigma_{bb}\,\Sigma_{\rm rr},
\qquad c_{b{\rm r}}=\max(8,\,4c_Y).
\]

\emph{(iv) $\hat\Sigma_{\rm rr}$.} $C^{\rm r}$ satisfies S2, so Propositions~\ref{prop:mean} and~\ref{prop:conc} apply to
$\hat\theta_{\rm r}$ and its variance estimate as they stand: $\E\hat\Sigma_{\rm rr}=\Sigma_{\rm rr}+b_{\rm r}(\Delta)$ and
$\E[(\hat\Sigma_{\rm rr}-\Sigma_{\rm rr}-b_{\rm r}(\Delta))^2]\le c'\bth^2(\max_U\phi^{\rm r}_U+\lambda_{\rm r}^2/\Sigma_{\rm rr})\,\Sigma_{\rm rr}^2$.

\emph{(v) The matrix.} Let $D=\diag(\Sigma_{bb}^{1/2},\Sigma_{\rm rr}^{1/2})$, $R=D^{-1}\Sigma D^{-1}$, the
correlation matrix with off-diagonal entry $\rho$, and $\hat R=D^{-1}\hat\Sigma^{\rm lo}D^{-1}$. Under the
conditions of the last part of the proposition, (i), (iii), (iv) and Chebyshev's inequality give $\hat R-R\to0$
entry by entry in probability. Now
$\Sigma^{-1/2}\hat\Sigma^{\rm lo}\Sigma^{-1/2}-I=\Sigma^{-1/2}D\,(\hat R-R)\,D\Sigma^{-1/2}$ and
$\|\Sigma^{-1/2}D\|_\op^2=\|D\Sigma^{-1}D\|_\op=\|R^{-1}\|_\op=1/(1-|\rho|)$, so
\[
\big\|\Sigma^{-1/2}\hat\Sigma^{\rm lo}\Sigma^{-1/2}-I\big\|\le\frac{\|\hat R-R\|}{1-|\rho|}\to0
\]
in probability when $|\rho|\le\bar\rho<1$.

\emph{(vi) Fitted values.} Let the movers' rows receive fitted values as in Proposition~\ref{prop:fitted}, and use
its notation: $v^*_i=\psi_i(\eta^*)$, $e_i=\hat v_i-v_i^*$, $\delta=\hat\eta-\eta^*$ and the event $\Omega$.
$\hat\Sigma_{bb}^{\rm lo}$ does not involve them. $\hat\Sigma_{\rm rr}$ is covered by Proposition~\ref{prop:fitted} applied to
$C^{\rm r}$. $\hat\Sigma_{b{\rm r}}$ is linear in the movers' values, and on $\Omega$
\[
\big|\hat\Sigma_{b{\rm r}}(\hat v)-\hat\Sigma_{b{\rm r}}(v^*)\big|=\Big|2\sum_{i\in R}\omega_i\hat m_i^{\rm r}e_i\Big|
\le2L\|\delta\|\Big(\sum_{i\in R}\omega_i^2\Big)^{1/2}\Big(\sum_{i\in R}(\hat m_i^{\rm r})^2\Big)^{1/2}
\le\frac{2L\|\delta\|}{\underline\sigma^{2}}\,\Sigma_{bb}^{1/2}\,\|w^{\rm r}_R\|,
\]
and $\E\|w^{\rm r}_R\|^2=\|a^{\rm r}_R\|^2+\|K^{\rm r}_{R\cdot}\|^2\le\frac12\Sigma_{\rm rr}$. By the Cauchy--Schwarz inequality
and $\E\|\delta\|^2\le c_\eta/n_0$, the expectation of the left-hand side over $\Omega$ is at most
$\sqrt2\,L\,c_\eta^{1/2}\underline\sigma^{-2}\,n_0^{-1/2}(\Sigma_{bb}\Sigma_{\rm rr})^{1/2}$. So the entry of $\hat R$ changes
by $O_p(n_0^{-1/2})$, and (v) applies with $\Delta_i=v_i^*-\sigma_i^2$. For the model of the text,
Corollary~\ref{cor:transfer} supplies the conditions with $n_0=n_{\cR}$.
\end{proof}

\paragraph{A sufficient condition for the remainder.} The remainder's weights are those of \eqref{eq:ustat} for
$\tilde A^{\rm r}=\tilde A-\lambda_1q_1q_1'$, whose non-zero eigenvalues are $\lambda_2,\dots,\lambda_r$. Lemma~\ref{lem:kss}
therefore applies to the remainder with $\lambda_2$ in place of $\lambda_1$, $\sum_{\ell\ge2}\lambda_\ell^2$ in place of
$\sum_\ell\lambda_\ell^2$, $\Sigma_{\rm rr}$ in place of $V$, and with $\tilde x_i$, $\check x_i$ formed from $A^{\rm r}$ and
$B^{\rm r}$ (KSS's $\tilde x_{i1}$ and $\check x_{i1}$): if the years per stayer are bounded,
$\lambda_2^2/\sum_{\ell\ge2}\lambda_\ell^2\to0$ and
$\Sigma_{\rm rr}^{-1}\max_i\big((\tilde x_{i1}'\beta)^2+(\check x_{i1}'\beta)^2\big)\to0$ imply
$\max_U\phi^{\rm r}_U+\lambda_{\rm r}^2/\Sigma_{\rm rr}\to0$. The first of these is stronger than the corresponding part of
KSS's Assumption~2, $\lambda_2^2/\sum_{\ell\ge1}\lambda_\ell^2\to0$: when $\lambda_1$ carries almost all of the sum, the
second eigenvalue can be negligible against the whole and still dominate the rest.

\paragraph{Units.} KSS's Lemma~7 asks for $\Sigma^{-1}\hat\Sigma\to I$. The interval is unchanged when $\hat b_1$ is
multiplied by a non-random number $s>0$ and $\lambda_1$ divided by $s^2$ (the ellipse and the function
$\lambda_1b^2+\theta_{\rm r}$ transform together, and $\hat\kappa$ does not change), and the conclusion of their
Theorem~3, which is about the standardised vector, is unchanged too. With $s^2=\Sigma_{\rm rr}/\Sigma_{bb}$ and
$T=\diag(s,1)$ the transformed matrices are $\Sigma^*=T\Sigma T=\Sigma_{\rm rr}R$ and
$\hat\Sigma^*=T\hat\Sigma^{\rm lo}T=\Sigma_{\rm rr}\hat R$, so $(\Sigma^*)^{-1}\hat\Sigma^*-I=R^{-1}(\hat R-R)$: for
$|\rho|\le\bar\rho<1$ their condition, in these units, is the entrywise convergence $\hat R-R\to0$ shown above.
In the original units $\Sigma^{-1}\hat\Sigma^{\rm lo}\to I$ need not hold when the two variances are of different
orders, and it is not needed.

\begin{proof}[Proof of Corollary~\ref{cor:q1}]
\emph{(a)} Write $\hat\Sigma$ for $\hat\Sigma^{\rm lo}$, positive definite with correlation $\hat\rho$,
$\hat\rho^2\le2/3$, and $\hat\Sigma^{+}$ for the same matrix with its first diagonal entry raised by
$\delta\ge0$, as \eqref{eq:q1est} does. The two intervals are ranges of the same function
$\lambda_1b^2+\theta_{\rm r}$ over two ellipses with the same centre $(\hat b_1,\hat\theta_{\rm r})$, so it suffices
that the ellipse of $\hat\Sigma^{+}$ contains that of $\hat\Sigma$; the sign of $\lambda_1$ plays no role, and for
$\lambda_1=0$ both curvatures are zero. $\hat\Sigma^{+}-\hat\Sigma$ is positive semidefinite, so
$x'(\hat\Sigma^{+})^{-1}x\le x'\hat\Sigma^{-1}x$ for every $x$. The curvature \eqref{eq:kappa} can be written
$\kappa=2|\lambda_1|\,\Sigma_{bb}^{3/2}/\det(\Sigma)^{1/2}$, and
\[
\frac{\partial\log\kappa}{\partial\Sigma_{bb}}=\frac{3}{2\,\Sigma_{bb}}-\frac{\Sigma_{\rm rr}}{2\det\Sigma}
=\frac{2\,\Sigma_{bb}\Sigma_{\rm rr}-3\,\Sigma_{b{\rm r}}^2}{2\,\Sigma_{bb}\det\Sigma},
\]
which is non-negative exactly when $\rho^2\le2/3$. Raising $\Sigma_{bb}$ lowers $\rho^2$, so the condition holds
along the way from $\hat\Sigma$ to $\hat\Sigma^{+}$, and $\hat\kappa^{+}\ge\hat\kappa$. For fixed $Z_1,Z_2$ the
quantity $(Z_1^2+(|Z_2|+r)^2)^{1/2}-r$ is non-increasing in $r=1/\kappa\ge0$ (its derivative is
$(|Z_2|+r)/(Z_1^2+(|Z_2|+r)^2)^{1/2}-1\le0$), so the critical value is non-decreasing in $\kappa$:
$z_{\alpha,\hat\kappa^{+}}\ge z_{\alpha,\hat\kappa}$. Hence the ellipse of $\hat\Sigma^{+}$ contains the ellipse of
$\hat\Sigma$. The bound is on the correlation before the entry is raised: if $\hat\rho^2>2/3$, raising the entry
can lower the curvature and with it the critical value, and an end point can move inwards.

\emph{(b)} By Proposition~\ref{prop:q1} and $|\rho|\le\bar\rho<(2/3)^{1/2}$, $\hat\Sigma^{\rm lo}$ is positive
definite with $\hat\rho_{\rm lo}^2\le2/3$ with probability tending to one. On that event the interval with
\eqref{eq:q1est} contains the interval with $\hat\Sigma^{\rm lo}$ by (a). Under the hypotheses of KSS's Theorem~3
and the consistency of $\hat\Sigma^{\rm lo}$ (in the units of the previous paragraph), their Lemma~7 gives the
latter a limiting coverage of at least $1-\alpha$. Hence the same bound holds for the former, whether or not
the first entry of \eqref{eq:q1est} is consistent.
\end{proof}

\subsection{What is not proved}

The propositions assume exact weights, a profile evaluated at fixed characteristics, known coefficients of
controls and a fixed centring constant. With projected leverages the weights have a small non-zero diagonal on
movers' rows, and no version of the propositions for these weights is proved here. Nor are: the profile evaluated
at estimated effects (the paragraph after Corollary~\ref{cor:transfer}); floors that bind in large samples; the
effects of the estimated mean and of estimated coefficients of controls; asymptotic normality of
$\hat\theta$, which is KSS's Theorem~2; and, for Section~\ref{sec:q1}, the joint normality of $\hat b_1$ and the
remainder and the coverage of the interval given a consistent covariance matrix, which are KSS's Theorem~3 and
Lemma~7. Corollary~\ref{cor:q1} needs the correlation of $\hat b_1$ and the remainder to stay below
$(2/3)^{1/2}$ in absolute value; nothing is shown for larger correlations, for a leading direction that rests on
a fixed handful of rows, or for an iteration that has not found the leading direction.
'''
A = re.sub(r'\bP([1-6])\b', r'S\1', A).replace('(P\\arabic*)', '(S\\arabic*)')
assert 'P\\arabic' not in A
open('a1_proofs.tex', 'w').write(A)
print('appendix written', len(A))
