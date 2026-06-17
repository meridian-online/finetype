# ac-01/02 recall-rule study (gold, baseline preds)

stats computed for 748/927 predicted gold columns

baseline categorical      TP=47 FP=7 FN=59 P=0.870 R=0.443
baseline alphanumeric_id  TP=40 FP=2 FN=19 P=0.952 R=0.678

## ac-02 alphanumeric_id — override TRIGGERS where high-card + alpha + digit
   (alnum = mixed letters+digits, near-unique; unknown = model gave up)
  unknown card>=0.9 a>=0.3 d>=0.3            fired=  0 recov=  0 newFP= 0 broke= 0 ovP=nan  R 0.678->0.678  P 0.952->0.952
  unknown card>=0.95 a>=0.5 d>=0.5           fired=  0 recov=  0 newFP= 0 broke= 0 ovP=nan  R 0.678->0.678  P 0.952->0.952
  unknown card>=0.99 a>=0.5 d>=0.3           fired=  0 recov=  0 newFP= 0 broke= 0 ovP=nan  R 0.678->0.678  P 0.952->0.952

  unknown+url card>=0.9 a>=0.3 d>=0.3        fired= 20 recov=  2 newFP=18 broke=18 ovP=0.10  R 0.678->0.712  P 0.952->0.677
  unknown+url card>=0.95 a>=0.5 d>=0.5       fired= 18 recov=  1 newFP=17 broke=17 ovP=0.06  R 0.678->0.695  P 0.952->0.683
  unknown+url card>=0.99 a>=0.5 d>=0.3       fired= 19 recov=  1 newFP=18 broke=18 ovP=0.05  R 0.678->0.695  P 0.952->0.672

  unknown+url+plain_text card>=0.9 a>=0.3 d>=0.3 fired= 22 recov=  2 newFP=20 broke=20 ovP=0.09  R 0.678->0.712  P 0.952->0.656
  unknown+url+plain_text card>=0.95 a>=0.5 d>=0.5 fired= 20 recov=  1 newFP=19 broke=19 ovP=0.05  R 0.678->0.695  P 0.952->0.661
  unknown+url+plain_text card>=0.99 a>=0.5 d>=0.3 fired= 19 recov=  1 newFP=18 broke=18 ovP=0.05  R 0.678->0.695  P 0.952->0.672

## ac-01 categorical — override TRIGGERS where low-card small vocab
   (entity_name/plain_text already corpus-honest-REFUTED; word already targeted)
  word card<=0.1 nd<=20                      fired=  4 recov=  3 newFP= 1 broke= 0 ovP=0.75  R 0.443->0.472  P 0.870->0.862
  word card<=0.3 nd<=30                      fired=  5 recov=  4 newFP= 1 broke= 0 ovP=0.80  R 0.443->0.481  P 0.870->0.864
  word card<=0.6 nd<=50                      fired=  5 recov=  4 newFP= 1 broke= 0 ovP=0.80  R 0.443->0.481  P 0.870->0.864

  word+ordinal card<=0.1 nd<=20              fired=  6 recov=  4 newFP= 2 broke= 0 ovP=0.67  R 0.443->0.481  P 0.870->0.850
  word+ordinal card<=0.3 nd<=30              fired=  8 recov=  6 newFP= 2 broke= 0 ovP=0.75  R 0.443->0.500  P 0.870->0.855
  word+ordinal card<=0.6 nd<=50              fired= 11 recov=  9 newFP= 2 broke= 0 ovP=0.82  R 0.443->0.528  P 0.870->0.862

  word+ordinal+entity_name+plain_text card<=0.1 nd<=20 fired=  9 recov=  7 newFP= 2 broke= 0 ovP=0.78  R 0.443->0.509  P 0.870->0.857
  word+ordinal+entity_name+plain_text card<=0.3 nd<=30 fired= 14 recov= 11 newFP= 3 broke= 0 ovP=0.79  R 0.443->0.547  P 0.870->0.853
  word+ordinal+entity_name+plain_text card<=0.6 nd<=50 fired= 19 recov= 16 newFP= 3 broke= 0 ovP=0.84  R 0.443->0.594  P 0.870->0.863

