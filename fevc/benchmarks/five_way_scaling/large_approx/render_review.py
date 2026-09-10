"""Presentation-only redraw; preserve the frozen run's plots and statistics."""
import json,sys
from pathlib import Path
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
from matplotlib.ticker import MaxNLocator
from matplotlib.lines import Line2D

run=Path(sys.argv[1]);out=run/'diagnostic'
stats=json.loads((out/'summary.json').read_text())['statistics']
oracle=json.loads((run/'input/oracle.json').read_text())
roles=('fevc','matlab','julia','r','pytwoway')
labels=('FEVC','KSS MATLAB','Julia','R','PyTwoWay')
fig,axes=plt.subplots(2,2,figsize=(11,7.5))
for ax,k in zip(axes.flat,('worker','firm','covariance','total')):
    for i,role in enumerate(roles):
        v=stats[role][k];m=v['median']
        ax.errorbar(m,i,xerr=[[m-v['minimum']],[v['maximum']-m]],fmt='o',capsize=3,color='#0072B2')
    ax.axvline(oracle['targets'][k],color='#009E73',linestyle='--')
    ax.set_yticks(range(5),labels);ax.invert_yaxis();ax.set_title(k.capitalize())
    ax.xaxis.set_major_locator(MaxNLocator(nbins=4));ax.ticklabel_format(axis='x',useOffset=False)
    ax.grid(axis='x',alpha=.2)
fig.suptitle('76,800 observations, 2 cores, 280 projections',fontsize=14,y=.985)
fig.legend([Line2D([],[],color='#0072B2',marker='o'),Line2D([],[],color='#009E73',linestyle='--')],
           ['Median and observed range','Exact reference'],loc='upper center',bbox_to_anchor=(.5,.956),ncol=2,frameon=False)
fig.text(.5,.02,'Ranges are not confidence intervals. MATLAB and Julia calls did not supply independent seed-controlled projections.',ha='center',fontsize=9)
fig.tight_layout(rect=(0,.06,1,.915))
for ext in ('png','svg'):fig.savefig(out/f'estimates_reviewed.{ext}',dpi=200,bbox_inches='tight')
plt.close(fig)
print('FEVC_LARGE_REVIEW_RENDER_PASS')
